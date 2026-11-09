//! Conservative, observed-damage crossbow saves. No changes to kill scoring.
use super::*;

const WINDOW_SECONDS: f64 = 1.0;
const MAX_SAMPLE_AGE_SECONDS: f64 = 0.25;
const SAVE_SCORE: f64 = 30.0;
const PROTECTION: [&str; 5] = [
    "invulnerable", "invulnerability_wearing_off", "uber_bullet_resist",
    "uber_blast_resist", "uber_fire_resist",
];

#[derive(Clone)]
struct Observed {
    tick: i64,
    state: Map<String, Value>,
    health: Option<i64>,
}

struct Pending {
    arrow: EventRecord,
    healer: i64,
    patient: i64,
    round: i64,
    healer_state: Map<String, Value>,
    identity: (i64, String, String, String),
    protection: [bool; 5],
    before: i64,
    healed: i64,
    deadline: i64,
    damage: Vec<Value>,
    decisive: Option<EventRecord>,
    decisive_health: Option<i64>,
}

struct Detector<'a> {
    rounds: &'a [Round],
    context: &'a DemoContext,
    tick_rate: f64,
    current: HashMap<i64, Observed>,
    entities: HashMap<i64, i64>,
    pending: HashMap<i64, Pending>,
    saves: Vec<Pending>,
    last_tick: i64,
}

fn identity(state: &Map<String, Value>) -> (i64, String, String, String) {
    (
        state.get("entity_id").and_then(Value::as_i64).unwrap_or_default(),
        state.get("steam_id").and_then(Value::as_str).unwrap_or_default().into(),
        canonical_team_value(state.get("team")),
        state.get("class").and_then(Value::as_str).unwrap_or_default().into(),
    )
}

fn protection(state: &Map<String, Value>) -> Option<[bool; 5]> {
    Some([
        state.get(PROTECTION[0])?.as_bool()?,
        state.get(PROTECTION[1])?.as_bool()?,
        state.get(PROTECTION[2])?.as_bool()?,
        state.get(PROTECTION[3])?.as_bool()?,
        state.get(PROTECTION[4])?.as_bool()?,
    ])
}

impl<'a> Detector<'a> {
    fn new(rounds: &'a [Round], context: &'a DemoContext, tick_rate: f64) -> Self {
        Self { rounds, context, tick_rate, current: HashMap::new(), entities: HashMap::new(),
               pending: HashMap::new(), saves: Vec::new(), last_tick: 0 }
    }

    fn clock(&mut self, tick: i64) {
        if tick <= 0 || tick < self.last_tick {
            self.current.clear();
            self.pending.clear();
            self.entities.clear();
        }
        self.last_tick = tick;
        self.pending.retain(|_, pending| tick <= pending.deadline);
    }

    fn invalidate(&mut self, user: i64) {
        self.pending.retain(|patient, p| *patient != user && p.healer != user);
        self.current.remove(&user);
    }

    fn fresh(&self, user: i64, tick: i64) -> Option<&Observed> {
        let observed = self.current.get(&user)?;
        (observed.health.is_some()
            && tick >= observed.tick
            && (tick - observed.tick) as f64 <= MAX_SAMPLE_AGE_SECONDS * self.tick_rate)
            .then_some(observed)
    }

    fn state(&mut self, record: &Value) {
        let tick = record.get("server_tick").and_then(Value::as_i64).unwrap_or_default();
        self.clock(tick);
        let mut updated = HashSet::new();
        for entity in record.get("removed_players").and_then(Value::as_array).into_iter().flatten() {
            if let Some(user) = entity.as_i64().and_then(|id| self.entities.remove(&id)) {
                self.invalidate(user);
            }
        }
        for player in record.get("players").and_then(Value::as_array).into_iter().flatten() {
            let Some(state) = player.as_object() else { continue };
            let entity = state.get("entity_id").and_then(Value::as_i64).unwrap_or_default();
            let user = state.get("user_id").and_then(Value::as_i64).unwrap_or_default();
            if let Some(previous) = self.entities.remove(&entity) {
                if previous != user { self.invalidate(previous); }
            }
            if entity <= 0 || user <= 0 { continue; }
            self.entities.insert(entity, user);
            let health = state.get("health").and_then(Value::as_i64).filter(|v| *v > 0);
            if state.get("in_pvs").and_then(Value::as_bool) != Some(true)
                || state.get("life_state").and_then(Value::as_str) != Some("alive")
                || health.is_none() || tick <= 0
            {
                self.invalidate(user);
                continue;
            }
            // An unaccounted health change, missing observation, or identity
            // transition makes the counterfactual unsafe. Re-sync for future arrows.
            if let Some(pending) = self.pending.get(&user) {
                let valid = self.fresh(user, tick).is_some_and(|previous| previous.health == health)
                    && identity(state) == pending.identity
                    && protection(state) == Some(pending.protection);
                if !valid { self.pending.remove(&user); }
            }
            if self.current.get(&user).is_some_and(|old| identity(&old.state) != identity(state)) {
                self.pending.retain(|_, p| p.healer != user);
            }
            self.current.insert(user, Observed { tick, state: state.clone(), health });
            updated.insert(user);
        }
        // Wait until the complete post-event packet state has been reconciled,
        // including any simultaneous death or healer/patient identity change.
        for user in updated {
            let confirmed = self.pending.get(&user).is_some_and(|p| {
                p.decisive.as_ref().is_some_and(|hit| {
                    record.get("packet_sequence").and_then(Value::as_i64)
                        .is_some_and(|packet| packet >= hit.packet_sequence)
                        && tick >= hit.analysis_tick()
                        && (tick - hit.analysis_tick()) as f64 <= MAX_SAMPLE_AGE_SECONDS * self.tick_rate
                })
            });
            if confirmed {
                self.saves.push(self.pending.remove(&user).unwrap());
            }
        }
    }

    fn arrow(&mut self, event: &EventRecord) {
        let healer = event.int(&["healer"]);
        let patient = event.int(&["target"]);
        self.pending.remove(&patient); // Never credit two arrows for the same damage.
        let Some(tick) = event.server_tick.filter(|t| *t > 0) else { return };
        let amount = event.int(&["amount"]);
        if healer <= 0 || patient <= 0 || healer == patient || amount <= 0 { return; }
        if self.context.analysis_scope == "pov_player_only"
            && self.context.pov_player_user_id != Some(healer) { return; }
        let Some(round) = self.rounds.iter().find(|r| tick >= r.start && tick < r.end) else { return };
        let Some(h) = self.fresh(healer, tick) else { return };
        let Some(p) = self.fresh(patient, tick) else { return };
        let team = canonical_team_value(p.state.get("team"));
        if !matches!(team.as_str(), "red" | "blu")
            || canonical_team_value(h.state.get("team")) != team
            || h.state.get("class").and_then(Value::as_str).map(canonical_class).as_deref() != Some("medic")
        { return; }
        let Some(flags) = protection(&p.state) else { return };
        if flags[0] || flags[1] { return; }
        let before = p.health.unwrap();
        let maximum = p.state.get("max_health").and_then(Value::as_i64).unwrap_or_default();
        // SDK crossbow_heal.amount is attempted healing, not iActualHealed.
        let healed = amount.min((maximum - before).max(0));
        if healed <= 0 { return; }
        let pending = Pending {
            arrow: event.clone(), healer, patient, round: round.index, healer_state: h.state.clone(),
            identity: identity(&p.state), protection: flags, before, healed,
            deadline: (tick + (WINDOW_SECONDS * self.tick_rate).floor() as i64).min(round.end - 1),
            damage: Vec::new(), decisive: None, decisive_health: None,
        };
        self.current.get_mut(&patient).unwrap().health = Some(before + healed);
        self.pending.insert(patient, pending);
    }

    fn packet(&mut self, events: &[EventRecord]) {
        let Some(first) = events.first() else { return };
        let tick = first.server_tick.unwrap_or_default();
        self.clock(tick);
        // Canonicalize the generic healed + crossbow event pair to the earlier
        // heal position. Ambiguous pairings invalidate attribution.
        let mut arrows = BTreeMap::<usize, &EventRecord>::new();
        let mut skip = HashSet::new();
        let mut seen = HashSet::new();
        for (index, arrow) in events.iter().enumerate().filter(|(_, e)| e.event_type == "crossbow_heal") {
            let key = (arrow.int(&["healer"]), arrow.int(&["target"]), arrow.int(&["amount"]));
            skip.insert(index);
            if !seen.insert(key) { continue; }
            let paired = events.iter().enumerate().filter(|(_, e)| e.event_type == "player_healed"
                && e.int(&["healer"]) == key.0 && e.int(&["patient"]) == key.1
                && e.int(&["amount"]) == key.2).map(|(i, _)| i).collect::<Vec<_>>();
            if paired.len() > 1 {
                self.invalidate(key.1);
                continue;
            }
            let canonical = paired.first().copied().unwrap_or(index).min(index);
            if let Some(pair) = paired.first() { skip.insert(*pair); }
            arrows.insert(canonical, arrow);
        }
        for (index, event) in events.iter().enumerate() {
            if event.server_tick != first.server_tick { self.pending.clear(); self.current.clear(); return; }
            if let Some(arrow) = arrows.get(&index) { self.arrow(arrow); }
            if skip.contains(&index) { continue; }
            let user = event.int(&["user_id", "userid"]);
            match event.event_type.as_str() {
                "player_hurt" => {
                    let amount = event.int(&["damage_amount"]);
                    let health = event.int(&["health"]);
                    let previous = self.fresh(user, tick).and_then(|p| p.health);
                    if amount <= 0 { continue; }
                    if health <= 0 || previous.is_none_or(|hp| hp - amount != health) {
                        self.invalidate(user);
                        continue;
                    }
                    self.current.get_mut(&user).unwrap().health = Some(health);
                    if let Some(pending) = self.pending.get_mut(&user) {
                        pending.damage.push(json!({
                            "demo_tick":event.demo_tick,"server_tick":tick,
                            "damage_amount":amount,"health_after":health,
                            "weapon_id":event.int(&["weapon_id"]),"custom":event.int(&["custom"]),
                        }));
                        if pending.decisive.is_none() && health - pending.healed <= 0 {
                            pending.decisive = Some(event.clone());
                            pending.decisive_health = Some(health);
                        }
                    }
                }
                "player_healed" => self.invalidate(event.int(&["patient"])),
                "player_spawn" | "player_disconnect" | "player_team" | "player_changeclass" | "player_death" => self.invalidate(user),
                "player_invulned" => self.invalidate(user),
                "player_chargedeployed" => {
                    self.invalidate(event.int(&["target_id"]));
                    self.invalidate(user);
                }
                "round_start" | "teamplay_round_start" | "teamplay_round_active"
                    | "teamplay_round_win" | "teamplay_round_stalemate" => {
                        self.pending.clear(); self.current.clear(); self.entities.clear();
                    }
                _ => {}
            }
        }
    }
}

/// Add new non-kill candidates only; never modify an existing candidate.
pub(super) fn append(
    export: &Path, events: &[EventRecord], rounds: &[Round], context: &DemoContext,
    manifest: &Value, source_demo: &str, map_name: &str, candidates: &mut Vec<Candidate>,
    cancelled: Option<&AtomicBool>, governor: Option<&RuntimeGovernor>,
) -> Result<()> {
    if !events.iter().any(|e| {
        e.event_type == "crossbow_heal"
            && e.server_tick.is_some_and(|tick| rounds.iter().any(|r| tick >= r.start && tick < r.end))
            && (context.analysis_scope != "pov_player_only"
                || context.pov_player_user_id == Some(e.int(&["healer"])))
    }) { return Ok(()); }
    // Older exports remain usable. Do not guess seconds from an unrelated clock.
    let Some(interval) = manifest.get("server_tick_interval").and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v >= 0.001 && *v <= 0.1) else { return Ok(()) };
    let path = export.join("state_samples.ndjson");
    if !path.is_file() { return Ok(()); }
    check_runtime(cancelled, governor)?;
    let mut work = 0u64;
    let mut checkpoint = || -> Result<()> {
        if work % 1024 == 0 { check_runtime(cancelled, governor)?; }
        work += 1;
        Ok(())
    };
    let mut detector = Detector::new(rounds, context, 1.0 / interval);
    let mut lines = BufReader::new(File::open(path)?).lines();
    let mut read_state = || -> Result<Option<Value>> {
        for line in lines.by_ref() {
            let line = line?;
            if !line.trim().is_empty() { return Ok(Some(serde_json::from_str(&line)?)); }
        }
        Ok(None)
    };
    let mut state = read_state()?;
    let mut index = 0;
    let mut last_packet = -1;
    while index < events.len() {
        checkpoint()?;
        let packet = events[index].packet_sequence;
        if packet < last_packet {
            // No support for unsafe legacy ordering. Existing candidates stay intact.
            return Ok(());
        }
        last_packet = packet;
        while state.as_ref().is_some_and(|r| r.get("packet_sequence").and_then(Value::as_i64).unwrap_or(-1) < packet) {
            detector.state(state.as_ref().unwrap());
            state = read_state()?;
            checkpoint()?;
        }
        let end = index + events[index..].iter().take_while(|e| e.packet_sequence == packet).count();
        detector.packet(&events[index..end]);
        if state.as_ref().is_some_and(|r| r.get("packet_sequence").and_then(Value::as_i64) == Some(packet)) {
            detector.state(state.as_ref().unwrap());
            state = read_state()?;
        }
        index = end;
    }
    while let Some(record) = state.take() {
        if detector.pending.is_empty() { break; }
        checkpoint()?;
        detector.state(&record);
        state = read_state()?;
    }
    for save in detector.saves {
        let hit = save.decisive.as_ref().unwrap();
        let tick = save.arrow.demo_tick;
        let decisive = hit.demo_tick;
        if tick <= 0 || decisive < tick { continue; }
        let actual = save.decisive_health.unwrap();
        let description = format!(
            "A Medic's crossbow healed a teammate for {} effective health. Recorded damage within one second left the teammate alive but would have exhausted their health without that arrow.",
            save.healed
        );
        let mut cue_ticks = vec![tick, decisive];
        cue_ticks.dedup();
        let mut tag_groups = vec![TickTagGroup { demo_tick: tick, server_ticks: vec![save.arrow.analysis_tick()], tags: vec!["crossbow_save".into()] }];
        if decisive != tick {
            tag_groups.push(TickTagGroup { demo_tick: decisive, server_ticks: vec![hit.analysis_tick()], tags: vec!["crossbow_save".into()] });
        }
        candidates.push(Candidate {
            candidate_id: format!("crossbow-save-r{}-h{}-p{}-t{}-e{}",save.round,save.healer,save.patient,tick,save.arrow.packet_sequence),
            source_demo: source_demo.into(), map_name: map_name.into(), round_index: save.round,
            overall_score: SAVE_SCORE, attacker_user_id: save.healer, attacker_class: "medic".into(),
            attacker_team: canonical_team_value(save.healer_state.get("team")),
            clip_start_tick: (tick - PRE_ROLL).max(0), clip_end_tick: decisive + POST_ROLL,
            // Existing non-kill bookmarks use this same cue mechanism.
            point_of_kill_ticks: cue_ticks,
            tags: vec!["crossbow_save".into()], primary_tag: "crossbow_save".into(),
            tick_tags: tag_groups,
            metrics: json!({"kills":0,"crossbow_saves":1,"patient_user_id":save.patient,
                "pre_arrow_health":save.before,"effective_arrow_healing":save.healed,
                "actual_health_after_decisive_damage":actual,"no_arrow_health_after_decisive_damage":actual-save.healed,
                "decisive_damage_tick":decisive,"window_seconds":WINDOW_SECONDS,
                "detector_version":1,"counterfactual":"same_observed_damage_without_this_arrow"}),
            kills: vec![],
            score_breakdown: vec![json!({"reason":"crossbow_save","points":SAVE_SCORE,"event_tick":save.arrow.analysis_tick(),"demo_tick":tick})],
            demo_context: context.clone(),
            extra: Map::from_iter([
                ("description".into(), json!(description)),
                ("attacker_name".into(), save.healer_state.get("name").cloned().unwrap_or(Value::Null)),
                ("save_evidence".into(), json!({"arrow":save.arrow,"damage":save.damage,
                    "patient_user_id":save.patient,"post_damage_state_reconciled":true})),
            ]),
            ..Candidate::default()
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round() -> Round {
        Round { index:1,start:1,end:1000,round_active_tick:1,start_event:"test".into(),
            end_event:String::new(),winning_team:0,ready_up:false,red_ready_tick:None,
            blu_ready_tick:None,ready_restart_tick:None,countdown_tick:None,
            setup_finished_tick:None,activation_trigger:"test".into() }
    }

    fn player(user: i64, health: i64, max: i64) -> Value {
        json!({"entity_id":user+10,"user_id":user,"steam_id":format!("steam-{user}"),
            "name":format!("Player {user}"),"team":"red","class":if user==1 {"medic"} else {"soldier"},
            "health":health,"max_health":max,"life_state":"alive","in_pvs":true,
            "invulnerable":false,"invulnerability_wearing_off":false,
            "uber_bullet_resist":false,"uber_blast_resist":false,"uber_fire_resist":false})
    }
    fn state(packet: i64, tick: i64, hp: i64, max: i64) -> Value {
        json!({"packet_sequence":packet,"server_tick":tick,
            "players":[player(1,150,150),player(2,hp,max)],"removed_players":[]})
    }
    fn event(packet: i64, tick: i64, kind: &str, data: Value) -> EventRecord {
        EventRecord { packet_sequence:packet,server_tick:Some(tick),demo_tick:tick-50,tick,
            event_type:kind.into(),event:data.as_object().unwrap().clone(),..EventRecord::default() }
    }
    fn arrow() -> EventRecord {
        event(2,101,"crossbow_heal",json!({"healer":1,"target":2,"amount":75}))
    }
    fn hurt(packet: i64, tick: i64, damage: i64, hp: i64) -> EventRecord {
        event(packet,tick,"player_hurt",json!({"user_id":2,"attacker":3,"damage_amount":damage,
            "health":hp,"weapon_id":22,"custom":0}))
    }
    fn with_detector(check: impl FnOnce(&mut Detector)) {
        let rounds = vec![round()];
        let context = DemoContext::default();
        check(&mut Detector::new(&rounds,&context,100.0));
    }
    fn setup(d: &mut Detector, hp: i64, max: i64, amount: i64) {
        d.state(&state(1,100,hp,max));
        let mut heal=arrow();
        heal.event.insert("amount".into(),json!(amount));
        d.packet(&[heal]);
    }
    fn finish(d: &mut Detector, damage: i64, hp: i64, max: i64) {
        d.packet(&[hurt(3,110,damage,hp)]);
        d.state(&state(3,110,hp,max));
    }

    #[test]
    fn projectile_damage_would_kill_without_the_arrow() {
        with_detector(|d| {
            setup(d,40,150,75); finish(d,60,55,150);
            assert_eq!(d.saves.len(),1);
            assert_eq!(d.saves[0].before,40);
            assert_eq!(d.saves[0].healed,75);
            assert_eq!(d.saves[0].decisive_health,Some(55));
        });
    }

    #[test]
    fn afterburn_two_ticks_can_exhaust_no_arrow_health() {
        with_detector(|d| {
            setup(d,6,150,75);
            for tick in [120,140] { d.state(&state(tick, tick,81,150)); }
            d.packet(&[hurt(150,150,4,77)]);
            d.state(&state(150,150,77,150));
            assert!(d.saves.is_empty());
            for tick in [170,190] { d.state(&state(tick,tick,77,150)); }
            d.packet(&[hurt(200,200,4,73)]);
            d.state(&state(200,200,73,150));
            assert_eq!(d.saves.len(),1);
            assert_eq!(d.saves[0].decisive_health,Some(73));
        });
    }

    #[test]
    fn attempted_heal_is_capped_to_actual_available_health() {
        with_detector(|d| {
            setup(d,40,100,100); finish(d,60,40,100);
            assert_eq!(d.saves.len(),1);
            assert_eq!(d.saves[0].healed,60);
        });
    }

    #[test]
    fn harmless_damage_and_overheal_do_not_create_a_save() {
        with_detector(|d| { setup(d,100,150,30); finish(d,20,110,150); assert!(d.saves.is_empty()); });
        with_detector(|d| { setup(d,150,150,75); assert!(d.pending.is_empty()); });
    }

    #[test]
    fn paired_generic_heal_is_not_counted_twice() {
        with_detector(|d| {
            d.state(&state(1,100,40,150));
            let generic=event(2,101,"player_healed",json!({"healer":1,"patient":2,"amount":75}));
            d.packet(&[generic,arrow()]);
            finish(d,60,55,150);
            assert_eq!(d.saves.len(),1);
        });
    }

    #[test]
    fn duplicate_arrow_event_cannot_create_two_saves() {
        with_detector(|d| {
            d.state(&state(1,100,40,150));
            d.packet(&[arrow(),arrow()]);
            finish(d,60,55,150);
            assert_eq!(d.saves.len(),1);
        });
    }

    #[test]
    fn unknown_protection_and_stale_pre_health_are_rejected() {
        with_detector(|d| {
            let mut s=state(1,100,40,150);
            s["players"][1]["invulnerable"]=Value::Null;
            d.state(&s);d.packet(&[arrow()]);assert!(d.pending.is_empty());
        });
        with_detector(|d| {
            d.state(&state(1,50,40,150));d.packet(&[arrow()]);assert!(d.pending.is_empty());
        });
    }

    #[test]
    fn other_healing_or_unexplained_health_change_cancels_attribution() {
        with_detector(|d| {
            setup(d,40,150,75);
            d.packet(&[event(3,105,"player_healed",json!({"healer":4,"patient":2,"amount":20}))]);
            finish(d,60,75,150);assert!(d.saves.is_empty());
        });
        with_detector(|d| {
            setup(d,40,150,75);d.state(&state(3,105,125,150));
            finish(d,60,65,150);assert!(d.saves.is_empty());
        });
    }

    #[test]
    fn protection_change_death_or_identity_reuse_cancels() {
        with_detector(|d| {
            setup(d,40,150,75);
            let mut s=state(3,105,115,150);s["players"][1]["invulnerable"]=json!(true);
            d.state(&s);finish(d,60,55,150);assert!(d.saves.is_empty());
        });
        with_detector(|d| {
            setup(d,40,150,75);
            d.packet(&[event(3,105,"player_death",json!({"user_id":2,"attacker":3}))]);
            finish(d,60,55,150);assert!(d.saves.is_empty());
        });
        with_detector(|d| {
            setup(d,40,150,75);
            let mut s=state(3,105,115,150);s["players"][1]["steam_id"]=json!("replacement");
            d.state(&s);finish(d,60,55,150);assert!(d.saves.is_empty());
        });
    }

    #[test]
    fn damage_requires_consistent_health_and_post_hit_alive_sample() {
        with_detector(|d| {
            setup(d,40,150,75);finish(d,60,60,150);assert!(d.saves.is_empty());
        });
        with_detector(|d| {
            setup(d,40,150,75);d.packet(&[hurt(3,110,60,55)]);assert!(d.saves.is_empty());
        });
        with_detector(|d| {
            setup(d,40,150,75);finish(d,115,0,150);assert!(d.saves.is_empty());
        });
    }

    #[test]
    fn expired_or_round_crossing_damage_is_not_a_save() {
        with_detector(|d| {
            setup(d,40,150,75);
            d.packet(&[hurt(3,202,60,55)]);d.state(&state(3,202,55,150));
            assert!(d.saves.is_empty());
        });
        with_detector(|d| {
            setup(d,40,150,75);
            d.packet(&[event(3,105,"teamplay_round_win",json!({"team":2}))]);
            finish(d,60,55,150);assert!(d.saves.is_empty());
        });
    }

    #[test]
    fn healer_pov_scope_is_preserved() {
        let rounds=vec![round()];
        let ctx=DemoContext { analysis_scope:"pov_player_only".into(),
            pov_player_user_id:Some(3),..DemoContext::default() };
        let mut d=Detector::new(&rounds,&ctx,100.0);
        setup(&mut d,40,150,75);finish(&mut d,60,55,150);
        assert!(d.saves.is_empty());
    }

    #[test]
    fn damage_before_arrow_does_not_credit_the_later_heal() {
        with_detector(|d| {
            d.state(&state(1,100,40,150));
            d.packet(&[hurt(2,101,20,20),arrow()]);
            d.state(&state(2,101,95,150));
            assert!(d.saves.is_empty());
        });
    }

    #[test]
    fn overlapping_arrows_do_not_both_claim_same_damage() {
        with_detector(|d| {
            setup(d,40,150,50);
            let next=event(3,105,"crossbow_heal",json!({"healer":1,"target":2,"amount":50}));
            d.packet(&[next]); // 90 -> 140, earlier window cancelled
            d.packet(&[hurt(4,110,100,40)]);d.state(&state(4,110,40,150));
            assert_eq!(d.saves.len(),1);
            assert_eq!(d.saves[0].before,90);
            assert_eq!(d.saves[0].healed,50);
        });
    }

    #[test]
    fn missing_server_clock_and_enemy_patient_do_not_start() {
        with_detector(|d| {
            d.state(&state(1,100,40,150));
            let mut heal=arrow();heal.server_tick=None;
            d.packet(&[heal]);assert!(d.pending.is_empty());
        });
        with_detector(|d| {
            let mut s=state(1,100,40,150);s["players"][1]["team"]=json!("blu");
            d.state(&s);d.packet(&[arrow()]);assert!(d.pending.is_empty());
        });
    }

    #[test]
    fn normal_analysis_adds_save_and_preserves_existing_bookmark() {
        let root=std::env::temp_dir().join(format!("arrow-save-integration-{}-{}",
            std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&root).unwrap();
        let mut events=vec![
            event(0,1,"teamplay_round_start",json!({})),
            event(0,2,"teamplay_round_active",json!({})),
            arrow(),hurt(3,110,60,55),
            event(4,900,"teamplay_round_win",json!({"team":2})),
        ];
        // Generic healed duplicate occurs first in the same packet.
        events.insert(2,event(2,101,"player_healed",json!({"healer":1,"patient":2,"amount":75})));
        let lines=events.iter().map(|e|serde_json::to_string(e).unwrap()).collect::<Vec<_>>().join("\n");
        fs::write(root.join("events.ndjson"),lines).unwrap();
        fs::write(root.join("state_samples.ndjson"),format!("{}\n{}\n",state(1,100,40,150),state(3,110,55,150))).unwrap();
        fs::write(root.join("header.json"),r#"{"map":"cp_test","ticks":1000}"#).unwrap();
        fs::write(root.join("bookmarks.json"),r#"{"bookmarks":[{"tick":400,"comment":"original bookmark"}]}"#).unwrap();
        fs::write(root.join("manifest.json"),r#"{"source_demo":"source.dem","demo_capture":{"classification":"stv"}}"#).unwrap();
        let original=analyze_export(&root,None).unwrap();
        assert_eq!(original.len(),1); // legacy export still works, no guessed interval
        let original=serde_json::to_value(&original[0]).unwrap();
        fs::write(root.join("manifest.json"),r#"{"source_demo":"source.dem","server_tick_interval":0.01,"demo_capture":{"classification":"stv"}}"#).unwrap();
        let enabled=analyze_export(&root,None).unwrap();
        assert_eq!(enabled.len(),2);
        let save=enabled.iter().find(|c| c.tags.iter().any(|t|t=="crossbow_save")).unwrap();
        assert_eq!(save.kill_count(),0);
        assert_eq!(save.attacker_user_id,1);
        assert_eq!(save.point_of_kill_ticks,vec![51,60]);
        assert_eq!(save.metrics["no_arrow_health_after_decisive_damage"],json!(-20));
        assert_eq!(save.overall_score,SAVE_SCORE);
        let bookmark=enabled.iter().find(|c|c.bookmark_tick.is_some()).unwrap();
        assert_eq!(serde_json::to_value(bookmark).unwrap(),original);
        let serialized=fs::read_to_string(root.join("frag_candidates.ndjson")).unwrap();
        assert!(serialized.contains("crossbow_save"));
        fs::remove_dir_all(root).unwrap();
    }
}
