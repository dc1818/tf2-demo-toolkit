//! Supported Uber rescues: unambiguous blocked-stat attribution plus outcome.
//! No weapon damage estimates and no changes to existing candidate scoring.
use super::*;
use std::collections::VecDeque;

const STAT_DELAY_SECONDS: f64 = 1.5; // SDK sends round stats once per second.
const FRESH_SECONDS: f64 = 0.25;
const LINGER_SECONDS: f64 = 1.0;
const OUTCOME_SECONDS: f64 = 8.0;
const SAVE_SCORE: f64 = 30.0;

type Identity = (i64, String, String, String);

#[derive(Clone)]
struct Snapshot {
    tick: i64,
    demo: i64,
    state: Map<String, Value>,
}
impl Snapshot {
    fn user(&self) -> i64 {
        number(&self.state, "user_id")
    }
    fn identity(&self) -> Identity {
        (
            number(&self.state, "entity_id"),
            string(&self.state, "steam_id"),
            canonical_team_value(self.state.get("team")),
            string(&self.state, "class"),
        )
    }
    fn health(&self) -> i64 {
        number(&self.state, "health")
    }
    fn alive(&self) -> bool {
        self.health() > 0
            && self.state.get("in_pvs").and_then(Value::as_bool) == Some(true)
            && string(&self.state, "life_state") == "alive"
    }
    fn protected(&self) -> bool {
        self.state.get("invulnerable").and_then(Value::as_bool) == Some(true)
            || self
                .state
                .get("invulnerability_wearing_off")
                .and_then(Value::as_bool)
                == Some(true)
    }
}
fn number(state: &Map<String, Value>, key: &str) -> i64 {
    state.get(key).and_then(Value::as_i64).unwrap_or_default()
}
fn string(state: &Map<String, Value>, key: &str) -> String {
    state
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .into()
}

#[derive(Clone)]
struct Deployment {
    event: EventRecord,
    healer: Snapshot,
    round: i64,
}
#[derive(Clone)]
struct Hit {
    event: EventRecord,
    patient: Snapshot,
    health: i64,
    providers: Vec<i64>,
    valid: bool,
}
#[derive(Clone)]
struct Outcome {
    event: EventRecord,
    actor: Option<Identity>,
    actor_user: i64,
    team: String,
    kind: String,
    survivors: HashMap<i64, Identity>,
}
struct Proof {
    deploy: Deployment,
    patient: Snapshot,
    first_hit: EventRecord,
    last_hit: EventRecord,
    confirmed_tick: i64,
    confirmed_demo: i64,
    blocked: i64,
    counter_before: i64,
    counter_after: i64,
    method: String,
    health_after: i64,
    required_health: i64,
    outcomes: Vec<Outcome>,
    deadline: i64,
    protected_hits: Vec<Hit>,
    provider_at_confirmation: Snapshot,
}

struct Detector<'a> {
    rate: f64,
    rounds: &'a [Round],
    context: &'a DemoContext,
    current: HashMap<i64, Snapshot>,
    entities: HashMap<i64, i64>,
    history: HashMap<i64, VecDeque<Snapshot>>,
    counters: HashMap<i64, (Identity, i64, i64)>,
    unsettled_hits: HashMap<i64, i64>,
    deployments: HashMap<i64, Deployment>,
    links: HashMap<(i64, i64), i64>,
    beams: HashMap<(i64, i64), i64>,
    hits: VecDeque<Hit>,
    damage: VecDeque<EventRecord>,
    boundaries: VecDeque<(i64, i64)>,
    recent_outcomes: VecDeque<Outcome>,
    pending: Vec<Proof>,
    finished: Vec<Proof>,
    proven: HashSet<(i64, i64)>,
    last_tick: i64,
}

impl<'a> Detector<'a> {
    fn new(rate: f64, rounds: &'a [Round], context: &'a DemoContext) -> Self {
        Self {
            rate,
            rounds,
            context,
            current: HashMap::new(),
            entities: HashMap::new(),
            history: HashMap::new(),
            counters: HashMap::new(),
            unsettled_hits: HashMap::new(),
            deployments: HashMap::new(),
            links: HashMap::new(),
            beams: HashMap::new(),
            hits: VecDeque::new(),
            damage: VecDeque::new(),
            boundaries: VecDeque::new(),
            recent_outcomes: VecDeque::new(),
            pending: Vec::new(),
            finished: Vec::new(),
            proven: HashSet::new(),
            last_tick: 0,
        }
    }
    fn fresh(&self, user: i64, tick: i64) -> Option<&Snapshot> {
        self.current.get(&user).filter(|s| {
            s.alive() && tick >= s.tick && (tick - s.tick) as f64 <= self.rate * FRESH_SECONDS
        })
    }
    fn finish_user(&mut self, user: i64, tick: i64) {
        self.boundaries.push_back((tick, user));
        let entity = self.current.get(&user).map(|s| s.identity().0);
        self.current.remove(&user);
        self.history.remove(&user);
        self.counters.remove(&user);
        self.unsettled_hits.remove(&user);
        self.deployments.remove(&user);
        self.links
            .retain(|(medic, patient), _| *medic != user && *patient != user);
        self.beams
            .retain(|(medic, patient), _| *medic != user && Some(*patient) != entity);
        // A death after a meaningful outcome does not erase the earlier rescue.
        let mut i = 0;
        while i < self.pending.len() {
            if self.pending[i].patient.user() == user {
                let p = self.pending.remove(i);
                if !p.outcomes.is_empty() {
                    self.finished.push(p);
                }
            } else {
                i += 1;
            }
        }
    }
    fn close_round(&mut self) {
        for p in self.pending.drain(..) {
            if !p.outcomes.is_empty() {
                self.finished.push(p);
            }
        }
        self.current.clear();
        self.entities.clear();
        self.history.clear();
        self.counters.clear();
        self.unsettled_hits.clear();
        self.deployments.clear();
        self.links.clear();
        self.beams.clear();
        self.hits.clear();
        self.damage.clear();
        self.boundaries.clear();
        self.recent_outcomes.clear();
        self.proven.clear();
    }
    fn clock(&mut self, tick: i64) {
        if tick <= 0 {
            self.close_round();
            self.last_tick = 0;
            return;
        }
        if tick < self.last_tick {
            self.close_round();
        }
        self.last_tick = tick;
        let earliest = tick - (self.rate * (STAT_DELAY_SECONDS + 1.0)).ceil() as i64;
        while self
            .hits
            .front()
            .is_some_and(|h| h.event.analysis_tick() < earliest)
        {
            self.hits.pop_front();
        }
        while self
            .damage
            .front()
            .is_some_and(|e| e.analysis_tick() < earliest)
        {
            self.damage.pop_front();
        }
        while self.boundaries.front().is_some_and(|(t, _)| *t < earliest) {
            self.boundaries.pop_front();
        }
        while self
            .recent_outcomes
            .front()
            .is_some_and(|e| e.event.analysis_tick() < tick - (self.rate * OUTCOME_SECONDS) as i64)
        {
            self.recent_outcomes.pop_front();
        }
        for h in self.history.values_mut() {
            while h.front().is_some_and(|s| s.tick < earliest) {
                h.pop_front();
            }
        }
        self.deployments
            .retain(|_, d| tick - d.event.analysis_tick() <= (self.rate * 9.5) as i64);
        self.links
            .retain(|_, t| tick - *t <= (self.rate * 9.5) as i64);
        self.beams
            .retain(|_, t| tick - *t <= (self.rate * LINGER_SECONDS) as i64);
        let mut i = 0;
        while i < self.pending.len() {
            if tick > self.pending[i].deadline {
                let p = self.pending.remove(i);
                if !p.outcomes.is_empty() {
                    self.finished.push(p);
                }
            } else {
                i += 1;
            }
        }
    }
    fn outcome_matches(p: &Proof, o: &Outcome) -> bool {
        let t = o.event.analysis_tick();
        t >= p.last_hit.analysis_tick()
            && t <= p.deadline
            && o.team == p.patient.identity().2
            && o.survivors.get(&p.patient.user()) == Some(&p.patient.identity())
            && (o.kind != "recipient_kill"
                || (o.actor_user == p.patient.user()
                    && o.actor.as_ref() == Some(&p.patient.identity())))
    }
    fn outcome(&mut self, event: &EventRecord, kind: &str, actor_user: i64, team: String) {
        let tick = event.analysis_tick();
        let actor = self.fresh(actor_user, tick).map(Snapshot::identity);
        let survivors = self
            .current
            .iter()
            .filter_map(|(u, _)| self.fresh(*u, tick).map(|s| (*u, s.identity())))
            .collect();
        let o = Outcome {
            event: event.clone(),
            actor,
            actor_user,
            team,
            kind: kind.into(),
            survivors,
        };
        for p in &mut self.pending {
            let survivor = self.current.get(&p.patient.user()).is_some_and(|s| {
                s.alive()
                    && s.identity() == p.patient.identity()
                    && tick >= s.tick
                    && (tick - s.tick) as f64 <= FRESH_SECONDS * self.rate
            });
            if survivor && Self::outcome_matches(p, &o) {
                p.outcomes.push(o.clone());
            }
        }
        self.recent_outcomes.push_back(o);
    }
    fn mark_unsettled(&mut self, tick: i64, patient: Option<&Snapshot>, providers: &[i64]) {
        for (medic, deploy) in &self.deployments {
            let team = patient.map(|p| p.identity().2).unwrap_or_default();
            if (team.is_empty() || team == deploy.healer.identity().2)
                && (providers.is_empty() || providers.contains(medic))
            {
                self.unsettled_hits.entry(*medic).or_insert(tick);
            }
        }
    }
    fn packet(&mut self, events: &[EventRecord]) {
        let Some(first) = events.first() else { return };
        let Some(tick) = first.server_tick.filter(|t| *t > 0) else {
            self.close_round();
            return;
        };
        self.clock(tick);
        if events.iter().any(|e| e.server_tick != Some(tick)) {
            self.close_round();
            return;
        }
        // Register deployment and provider links before same-packet hit events;
        // SDK event order is not guaranteed.
        for e in events
            .iter()
            .filter(|e| e.event_type == "player_chargedeployed")
        {
            let medic = e.int(&["user_id", "userid"]);
            let Some(healer) = self.fresh(medic, tick).cloned() else {
                continue;
            };
            if e.demo_tick <= 0
                || canonical_class(&string(&healer.state, "class")) != "medic"
                || string(&healer.state, "medigun") != "uber"
            {
                continue;
            }
            if self.context.analysis_scope == "pov_player_only"
                && self.context.pov_player_user_id != Some(medic)
            {
                continue;
            }
            let Some(round) = self.rounds.iter().find(|r| tick >= r.start && tick < r.end) else {
                continue;
            };
            self.deployments.insert(
                medic,
                Deployment {
                    event: e.clone(),
                    healer,
                    round: round.index,
                },
            );
        }
        for e in events.iter().filter(|e| e.event_type == "player_invulned") {
            self.links.insert(
                (e.int(&["medic_user_id"]), e.int(&["user_id", "userid"])),
                tick,
            );
        }
        for e in events {
            let user = e.int(&["user_id", "userid"]);
            match e.event_type.as_str() {
                "player_hurt" if e.int(&["damage_amount"]) == 0 => {
                    let snapshot = self.current.get(&user).cloned();
                    let valid = e.int(&["health"]) > 0
                        && self.fresh(user, tick).is_some_and(Snapshot::protected);
                    if let Some(patient) = snapshot {
                        let mut providers = self
                            .deployments
                            .iter()
                            .filter_map(|(medic, d)| {
                                let same_team = d.healer.identity().2 == patient.identity().2;
                                let named =
                                    self.links.contains_key(&(*medic, user)) || *medic == user;
                                let beam = *medic == user
                                    || self.beams.get(&(*medic, patient.identity().0)).is_some_and(
                                        |t| {
                                            tick >= *t
                                                && (tick - *t) as f64 <= self.rate * LINGER_SECONDS
                                        },
                                    );
                                (same_team && named && beam).then_some(*medic)
                            })
                            .collect::<Vec<_>>();
                        providers.sort_unstable();
                        self.mark_unsettled(tick, Some(&patient), &providers);
                        let mut at_hit = patient.clone();
                        at_hit.tick = tick;
                        at_hit.demo = e.demo_tick;
                        at_hit
                            .state
                            .insert("health".into(), json!(e.int(&["health"])));
                        self.history
                            .entry(user)
                            .or_default()
                            .push_back(at_hit.clone());
                        self.hits.push_back(Hit {
                            event: e.clone(),
                            patient: at_hit,
                            health: e.int(&["health"]),
                            providers,
                            valid,
                        });
                    } else {
                        self.mark_unsettled(tick, None, &[]);
                        // An unmapped recipient can belong to any provider.
                        self.hits.push_back(Hit {
                            event: e.clone(),
                            patient: Snapshot {
                                tick,
                                demo: e.demo_tick,
                                state: Map::new(),
                            },
                            health: e.int(&["health"]),
                            providers: vec![],
                            valid: false,
                        });
                    }
                }
                "player_hurt" if e.int(&["damage_amount"]) > 0 => {
                    self.damage.push_back(e.clone());
                    // Damage that bypassed invulnerability (or has unknown
                    // protection provenance) may also contribute to a provider
                    // total. Never allocate that aggregate to zero-damage hits.
                    let snapshot = self.current.get(&user).cloned();
                    let known_unprotected = snapshot.as_ref().is_some_and(|s| {
                        s.state.get("invulnerable").and_then(Value::as_bool) == Some(false)
                            && s.state
                                .get("invulnerability_wearing_off")
                                .and_then(Value::as_bool)
                                == Some(false)
                    });
                    if !known_unprotected {
                        self.mark_unsettled(tick, snapshot.as_ref(), &[]);
                        self.hits.push_back(Hit {
                            event: e.clone(),
                            patient: snapshot.unwrap_or(Snapshot {
                                tick,
                                demo: e.demo_tick,
                                state: Map::new(),
                            }),
                            health: e.int(&["health"]),
                            providers: vec![],
                            valid: false,
                        });
                    }
                }
                "player_death" => {
                    let attacker = e.int(&["attacker"]);
                    let actor = self.fresh(attacker, tick).cloned();
                    let victim = self.current.get(&user);
                    if attacker > 0
                        && attacker != user
                        && actor.as_ref().zip(victim).is_some_and(|(a, v)| {
                            matches!(a.identity().2.as_str(), "red" | "blu")
                                && matches!(v.identity().2.as_str(), "red" | "blu")
                                && a.identity().2 != v.identity().2
                        })
                    {
                        self.outcome(e, "recipient_kill", attacker, actor.unwrap().identity().2);
                    }
                    self.finish_user(user, tick);
                }
                "teamplay_point_captured" | "payload_pushed" | "teamplay_capture_blocked" => {
                    let actor = e.int(&["pusher", "blocker"]);
                    let team = match e.int(&["team", "team_id"]) {
                        2 => "red".into(),
                        3 => "blu".into(),
                        _ => self
                            .current
                            .get(&actor)
                            .map(|s| s.identity().2)
                            .unwrap_or_default(),
                    };
                    if !team.is_empty() {
                        self.outcome(e, "team_objective_followup", actor, team);
                    }
                }
                "teamplay_round_win" => {
                    let team = match e.int(&["team"]) {
                        2 => "red",
                        3 => "blu",
                        _ => "",
                    };
                    if !team.is_empty() {
                        self.outcome(e, "team_round_win_followup", 0, team.into());
                    }
                    self.close_round();
                }
                "round_start"
                | "teamplay_round_start"
                | "teamplay_round_active"
                | "teamplay_round_stalemate"
                | "teamplay_game_over" => self.close_round(),
                "player_spawn" | "player_disconnect" | "player_team" | "player_changeclass" => {
                    self.finish_user(user, tick)
                }
                _ => {}
            }
        }
    }
    fn state(&mut self, record: &Value) {
        let tick = record
            .get("server_tick")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        self.clock(tick);
        if tick <= 0 {
            return;
        }
        let demo = record
            .get("demo_tick")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        for entity in record
            .get("removed_players")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_i64)
        {
            if let Some(user) = self.entities.remove(&entity) {
                self.finish_user(user, tick);
            }
        }
        let mut changes = Vec::new();
        let mut settled = HashSet::new();
        for p in record
            .get("players")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_object)
        {
            let user = number(p, "user_id");
            let entity = number(p, "entity_id");
            if user <= 0 || entity <= 0 {
                continue;
            }
            if let Some(old) = self.entities.insert(entity, user) {
                if old != user {
                    self.finish_user(old, tick);
                }
            }
            let snapshot = Snapshot {
                tick,
                demo,
                state: p.clone(),
            };
            if self
                .current
                .get(&user)
                .is_some_and(|old| old.identity() != snapshot.identity())
            {
                self.finish_user(user, tick);
            }
            if !snapshot.alive() {
                self.finish_user(user, tick);
            }
            if self.deployments.contains_key(&user) && string(p, "medigun") != "uber" {
                self.finish_user(user, tick);
            }
            if p.get("medigun_target").is_none_or(Value::is_null) {
                self.beams.retain(|(medic, _), _| *medic != user);
            }
            if let Some(target) = p
                .get("medigun_target")
                .and_then(Value::as_object)
                .and_then(|t| t.get("entity_id"))
                .and_then(Value::as_i64)
                .filter(|id| *id > 0)
            {
                if snapshot.alive() {
                    self.beams.insert((user, target), tick);
                }
            }
            self.history
                .entry(user)
                .or_default()
                .push_back(snapshot.clone());
            if let Some(value) = p
                .get("damage_blocked_total")
                .and_then(Value::as_i64)
                .filter(|v| *v >= 0)
            {
                if self
                    .counters
                    .get(&user)
                    .is_none_or(|(_, previous, _)| value != *previous)
                {
                    settled.insert(user);
                }
                if self
                    .counters
                    .get(&user)
                    .is_some_and(|(_, previous, _)| value < *previous)
                {
                    // A reset can mark a round epoch even when its event is
                    // missing. Preserve completed rescues, never future outcomes.
                    let mut i = 0;
                    while i < self.pending.len() {
                        if self.pending[i].deploy.healer.user() == user {
                            let p = self.pending.remove(i);
                            if !p.outcomes.is_empty() {
                                self.finished.push(p);
                            }
                        } else {
                            i += 1;
                        }
                    }
                    self.finish_user(user, tick);
                }
                if let Some((identity, previous, seen)) = self.counters.get(&user) {
                    if identity == &snapshot.identity()
                        && value > *previous
                        && tick >= *seen
                        && (tick - *seen) as f64 <= self.rate * FRESH_SECONDS
                    {
                        changes.push((user, *previous, value, snapshot.clone()));
                    }
                }
                self.counters
                    .insert(user, (snapshot.identity(), value, tick));
            } else {
                self.counters.remove(&user);
                self.unsettled_hits.remove(&user);
            }
            self.current.insert(user, snapshot);
        }
        // Resolve only after all post-packet patient/provider snapshots arrived.
        for (medic, before, after, snapshot) in changes {
            self.counter(medic, before, after, &snapshot);
        }
        for medic in settled {
            self.unsettled_hits.remove(&medic);
        }
    }
    fn counter(&mut self, medic: i64, before: i64, after: i64, provider: &Snapshot) {
        let Some(deploy) = self.deployments.get(&medic).cloned() else {
            return;
        };
        if !provider.alive()
            || provider.identity() != deploy.healer.identity()
            || string(&provider.state, "medigun") != "uber"
        {
            return;
        }
        let tick = provider.tick;
        let start = tick - (self.rate * STAT_DELAY_SECONDS).ceil() as i64;
        // A delayed counter must not silently attribute older, no-longer
        // buffered damage to the latest hit. Keep this marker until a real
        // counter change acknowledges it; repeated cached values do not.
        if self
            .unsettled_hits
            .get(&medic)
            .is_some_and(|first| *first <= start)
        {
            return;
        }
        // All potentially contributing protected hits on the provider's team
        // must identify the same recipient and exclusive provider.
        let relevant = self
            .hits
            .iter()
            .filter(|h| {
                h.event.analysis_tick() > start
                    && h.event.analysis_tick() <= tick
                    && (h.patient.identity().2.is_empty()
                        || h.patient.identity().2 == deploy.healer.identity().2)
            })
            .collect::<Vec<_>>();
        let Some(first) = relevant.first() else {
            return;
        };
        let patient = first.patient.user();
        if patient <= 0
            || relevant.iter().any(|h| {
                !h.valid
                    || h.patient.user() != patient
                    || h.patient.identity() != first.patient.identity()
                    || h.providers != vec![medic]
            })
        {
            return;
        }
        if first.event.analysis_tick() < deploy.event.analysis_tick() {
            return;
        }
        if self
            .proven
            .contains(&(deploy.event.packet_sequence, patient))
        {
            return;
        }
        if self
            .boundaries
            .iter()
            .any(|(t, u)| *t >= start && (*u == patient || *u == medic))
        {
            return;
        }
        let Some(current) = self.fresh(patient, tick).cloned() else {
            return;
        };
        if current.identity() != first.patient.identity() {
            return;
        }
        let blocked = after - before;
        let Some(direct_required) = relevant
            .iter()
            .try_fold(0i64, |sum, h| sum.checked_add(h.health))
        else {
            return;
        };
        // Pigeonhole bound: if total damage >= sum of actual HP at every
        // possible contributing hit, at least one hit was individually lethal.
        let (method, required) = if blocked >= direct_required {
            ("individual_lethal_hit_bound", direct_required)
        } else {
            // Cumulative bound is valid only with a fully observed monotone
            // HP curve strictly below the healing cap and no actual damage.
            let first_tick = first.event.analysis_tick();
            if self.damage.iter().any(|e| {
                e.int(&["user_id", "userid"]) == patient
                    && e.analysis_tick() >= first_tick
                    && e.analysis_tick() <= tick
            }) {
                return;
            }
            let mut curve = self
                .history
                .get(&patient)
                .into_iter()
                .flatten()
                .filter(|s| s.tick >= first_tick && s.tick <= tick)
                .collect::<Vec<_>>();
            curve.sort_by_key(|s| s.tick);
            let mut previous_tick = first_tick;
            let mut previous_health = first.health;
            for s in curve {
                if !s.alive()
                    || s.identity() != current.identity()
                    || (s.tick - previous_tick) as f64 > self.rate * FRESH_SECONDS
                    || s.health() < previous_health
                    || s.health() >= number(&s.state, "max_health")
                    || number(&s.state, "max_health") != number(&first.patient.state, "max_health")
                {
                    return;
                }
                previous_tick = s.tick;
                previous_health = s.health();
            }
            if (tick - previous_tick) as f64 > self.rate * FRESH_SECONDS
                || blocked < current.health()
            {
                return;
            }
            ("uncapped_observed_health_counterfactual", current.health())
        };
        let last = relevant.last().unwrap().event.clone();
        let first_event = first.event.clone();
        let mut proof = Proof {
            deploy,
            patient: current.clone(),
            first_hit: first_event,
            last_hit: last,
            confirmed_tick: tick,
            confirmed_demo: provider.demo,
            blocked,
            counter_before: before,
            counter_after: after,
            method: method.into(),
            health_after: current.health(),
            required_health: required,
            outcomes: vec![],
            protected_hits: relevant.iter().map(|h| (*h).clone()).collect(),
            provider_at_confirmation: provider.clone(),
            deadline: 0,
        };
        let round_end = self
            .rounds
            .iter()
            .find(|r| r.index == proof.deploy.round)
            .map(|r| r.end)
            .unwrap_or(tick);
        proof.deadline = (proof.first_hit.analysis_tick()
            + (self.rate * OUTCOME_SECONDS).floor() as i64)
            .min(round_end);
        for o in &self.recent_outcomes {
            if Self::outcome_matches(&proof, o) {
                proof.outcomes.push(o.clone());
            }
        }
        self.proven
            .insert((proof.deploy.event.packet_sequence, patient));
        self.pending.push(proof);
    }
    fn finish(mut self) -> Vec<Proof> {
        for p in self.pending.drain(..) {
            if !p.outcomes.is_empty() {
                self.finished.push(p);
            }
        }
        self.finished
    }
}

pub(super) fn append(
    export: &Path,
    events: &[EventRecord],
    rounds: &[Round],
    context: &DemoContext,
    manifest: &Value,
    source_demo: &str,
    map_name: &str,
    candidates: &mut Vec<Candidate>,
    cancelled: Option<&AtomicBool>,
    governor: Option<&RuntimeGovernor>,
) -> Result<()> {
    if manifest
        .get("parser_reported_incomplete")
        .and_then(Value::as_bool)
        == Some(true)
        || map_name.starts_with("mvm_")
        || rounds.is_empty()
        || !events
            .iter()
            .any(|e| e.event_type == "player_chargedeployed")
    {
        return Ok(());
    }
    let Some(interval) = manifest
        .get("server_tick_interval")
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v >= 0.001 && *v <= 0.1)
    else {
        return Ok(());
    };
    let path = export.join("state_samples.ndjson");
    if !path.is_file() {
        return Ok(());
    }
    let mut detector = Detector::new(1.0 / interval, rounds, context);
    let mut lines = BufReader::new(File::open(path)?).lines();
    let mut read = || -> Result<Option<Value>> {
        for line in lines.by_ref() {
            let line = line?;
            if !line.trim().is_empty() {
                return Ok(Some(serde_json::from_str(&line)?));
            }
        }
        Ok(None)
    };
    let mut state = read()?;
    let mut index = 0;
    let mut work = 0u64;
    let mut previous = -1;
    let mut checkpoint = || -> Result<()> {
        if work % 1024 == 0 {
            check_runtime(cancelled, governor)?;
        }
        work += 1;
        Ok(())
    };
    while index < events.len() {
        checkpoint()?;
        let packet = events[index].packet_sequence;
        if packet < previous {
            return Ok(());
        }
        previous = packet;
        while state.as_ref().is_some_and(|s| {
            s.get("packet_sequence")
                .and_then(Value::as_i64)
                .unwrap_or(-1)
                < packet
        }) {
            detector.state(state.as_ref().unwrap());
            state = read()?;
            checkpoint()?;
        }
        let end = index
            + events[index..]
                .iter()
                .take_while(|e| e.packet_sequence == packet)
                .count();
        detector.packet(&events[index..end]);
        if state
            .as_ref()
            .is_some_and(|s| s.get("packet_sequence").and_then(Value::as_i64) == Some(packet))
        {
            detector.state(state.as_ref().unwrap());
            state = read()?;
        }
        index = end;
    }
    while let Some(record) = state.take() {
        if detector.pending.is_empty() {
            break;
        }
        checkpoint()?;
        detector.state(&record);
        state = read()?;
    }
    for p in detector.finish() {
        let mut cues = vec![
            p.deploy.event.demo_tick,
            p.first_hit.demo_tick,
            p.last_hit.demo_tick,
            p.confirmed_demo,
        ];
        cues.extend(p.outcomes.iter().map(|o| o.event.demo_tick));
        cues.retain(|t| *t > 0);
        cues.sort_unstable();
        cues.dedup();
        if cues.is_empty() {
            continue;
        }
        let mut tags = vec!["uber_save".to_string()];
        tags.extend(p.outcomes.iter().map(|o| format!("uber_save_{}", o.kind)));
        tags.sort();
        tags.dedup();
        let outcomes = p
            .outcomes
            .iter()
            .map(|o| {
                json!({"kind":o.kind,"demo_tick":o.event.demo_tick,
            "server_tick":o.event.analysis_tick(),"actor_user_id":o.actor_user,"team":o.team})
            })
            .collect::<Vec<_>>();
        let first = *cues.first().unwrap();
        let last = *cues.last().unwrap();
        let patient_name = string(&p.patient.state, "name");
        let description=format!("A Medic's Uber protected {} from otherwise lethal blocked damage. The player survived and a kill or team objective followed. Attribution uses an unambiguous provider counter interval; followups describe observed events, not proof that the save caused the outcome.",if patient_name.is_empty(){format!("player {}",p.patient.user())}else{patient_name});
        candidates.push(Candidate{
            candidate_id:format!("uber-save-r{}-h{}-p{}-e{}",p.deploy.round,p.deploy.healer.user(),p.patient.user(),p.deploy.event.packet_sequence),
            source_demo:source_demo.into(),map_name:map_name.into(),round_index:p.deploy.round,
            overall_score:SAVE_SCORE,attacker_user_id:p.deploy.healer.user(),attacker_class:"medic".into(),
            attacker_team:p.deploy.healer.identity().2,clip_start_tick:(first-PRE_ROLL).max(0),clip_end_tick:last+POST_ROLL,
            point_of_kill_ticks:cues,
            tags:tags.clone(),primary_tag:"uber_save".into(),
            tick_tags:vec![TickTagGroup{demo_tick:p.deploy.event.demo_tick,server_ticks:vec![p.deploy.event.analysis_tick()],tags:vec!["uber_save".into()]}],
            sequence_tags:tags.into_iter().filter(|t|t!="uber_save").collect(),
            metrics:json!({"kills":0,"uber_saves":1,"patient_user_id":p.patient.user(),
                "blocked_damage":p.blocked,"counter_before":p.counter_before,"counter_after":p.counter_after,
                "confirmation_server_tick":p.confirmed_tick,"actual_health_at_confirmation":p.health_after,
                "lethality_bound_health":p.required_health,"proof_method":p.method,
                "counter_delay_bound_seconds":STAT_DELAY_SECONDS,"outcome_window_seconds":OUTCOME_SECONDS,
                "outcomes":outcomes,"detector_version":1}),
            score_breakdown:vec![json!({"reason":"uber_save","points":SAVE_SCORE,"event_tick":p.first_hit.analysis_tick()})],
            demo_context:context.clone(),
            extra:Map::from_iter([("description".into(),json!(description)),
                ("attacker_name".into(),p.deploy.healer.state.get("name").cloned().unwrap_or(Value::Null)),
                ("save_evidence".into(),json!({"deployment":p.deploy.event,
                    "first_protected_hit":p.first_hit,"last_protected_hit":p.last_hit,
                    "provider_user_id":p.deploy.healer.user(),"patient_user_id":p.patient.user(),
                    "counter_attribution":"single_recipient_single_provider",
                    "provider_at_deployment":p.deploy.healer.state,
                    "provider_at_confirmation":p.provider_at_confirmation.state,
                    "patient_at_confirmation":p.patient.state,
                    "protected_hits":p.protected_hits.iter().map(|h|json!({"event":h.event,
                        "actual_health":h.health,"patient_state":h.patient.state,
                        "possible_provider_user_ids":h.providers})).collect::<Vec<_>>(),
                    "counterfactual":"same_observed_health_and_actions_without_protection"}))]),
            ..Candidate::default()
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn round() -> Round {
        Round {
            index: 1,
            start: 100,
            end: 5000,
            round_active_tick: 100,
            start_event: "test".into(),
            end_event: "test".into(),
            winning_team: 0,
            ready_up: false,
            red_ready_tick: None,
            blu_ready_tick: None,
            ready_restart_tick: None,
            countdown_tick: None,
            setup_finished_tick: None,
            activation_trigger: "test".into(),
        }
    }
    fn player(user: i64, entity: i64, team: &str, hp: i64) -> Value {
        json!({"user_id":user,"entity_id":entity,"steam_id":format!("steam-{user}"),
            "name":format!("Player {user}"),"team":team,"class":if user==78||user==9 {"medic"}else{"soldier"},
            "health":hp,"max_health":300,"in_pvs":true,"life_state":"alive",
            "medigun":"uber","medigun_target":{"entity_id":6},"damage_blocked_total":140,
            "invulnerable":true,"invulnerability_wearing_off":false,
            "uber_bullet_resist":false,"uber_blast_resist":false,"uber_fire_resist":false})
    }
    fn state(tick: i64, hp: i64, counter: i64) -> Value {
        let mut medic = player(78, 7, "red", 150);
        medic["damage_blocked_total"] = json!(counter);
        json!({"packet_sequence":tick,"server_tick":tick,"demo_tick":tick-50,
            "players":[medic,player(62,6,"red",hp),player(72,20,"blu",100)],"removed_players":[]})
    }
    fn event(tick: i64, kind: &str, data: Value) -> EventRecord {
        EventRecord {
            server_tick: Some(tick),
            demo_tick: tick - 50,
            tick,
            packet_sequence: tick,
            event_type: kind.into(),
            event: data.as_object().unwrap().clone(),
            ..EventRecord::default()
        }
    }
    fn deploy(tick: i64) -> Vec<EventRecord> {
        vec![
            event(
                tick,
                "player_chargedeployed",
                json!({"user_id":78,"target_id":62}),
            ),
            event(
                tick,
                "player_invulned",
                json!({"user_id":62,"medic_user_id":78}),
            ),
        ]
    }
    fn hit(tick: i64, hp: i64) -> EventRecord {
        event(
            tick,
            "player_hurt",
            json!({"user_id":62,"attacker":72,"damage_amount":0,"health":hp}),
        )
    }
    fn kill(tick: i64) -> EventRecord {
        event(tick, "player_death", json!({"user_id":72,"attacker":62}))
    }
    fn with_detector(check: impl FnOnce(&mut Detector)) {
        let rounds = vec![round()];
        let ctx = DemoContext::default();
        check(&mut Detector::new(100.0, &rounds, &ctx));
    }
    fn setup(d: &mut Detector, hp: i64) {
        d.state(&state(200, hp, 140));
        d.packet(&deploy(201));
        d.packet(&[hit(202, hp)]);
    }
    fn confirm(d: &mut Detector, hp: i64, blocked: i64) {
        d.state(&state(210, hp, 140 + blocked));
    }
    fn output(d: &mut Detector) -> Vec<Proof> {
        let replacement = Detector::new(d.rate, d.rounds, d.context);
        std::mem::replace(d, replacement).finish()
    }
    #[test]
    fn lethal_blocked_hit_requires_followup_and_uses_user_not_entity() {
        with_detector(|d| {
            setup(d, 20);
            confirm(d, 20, 30);
            assert_eq!(d.pending.len(), 1);
            assert_eq!(d.pending[0].deploy.healer.user(), 78);
            assert_eq!(d.pending[0].patient.user(), 62);
            d.packet(&[kill(211)]);
            let p = output(d);
            assert_eq!(p.len(), 1);
            assert_eq!(p[0].outcomes[0].kind, "recipient_kill");
        });
        with_detector(|d| {
            setup(d, 20);
            confirm(d, 20, 30);
            assert!(output(d).is_empty());
        });
    }
    #[test]
    fn two_nonlethal_pipes_can_be_cumulatively_lethal() {
        with_detector(|d| {
            setup(d, 150);
            d.packet(&[hit(205, 150)]);
            confirm(d, 150, 200);
            assert_eq!(d.pending.len(), 1);
            assert_eq!(
                d.pending[0].method,
                "uncapped_observed_health_counterfactual"
            );
            d.packet(&[kill(211)]);
            assert_eq!(output(d).len(), 1);
        });
    }
    #[test]
    fn healing_is_included_in_cumulative_health_balance() {
        with_detector(|d| {
            setup(d, 150);
            d.state(&state(204, 180, 140));
            d.packet(&[hit(205, 180)]);
            confirm(d, 210, 200);
            assert!(d.pending.is_empty());
        });
        with_detector(|d| {
            setup(d, 150);
            d.state(&state(204, 160, 140));
            d.packet(&[hit(205, 160)]);
            confirm(d, 180, 200);
            assert_eq!(d.pending.len(), 1);
        });
    }
    #[test]
    fn capped_healing_or_unexplained_decrease_blocks_cumulative_proof() {
        for hp in [300, 140] {
            with_detector(|d| {
                setup(d, 150);
                d.state(&state(204, hp, 140));
                d.packet(&[hit(205, hp)]);
                confirm(d, hp, 200);
                assert!(d.pending.is_empty());
            });
        }
    }
    #[test]
    fn actual_damage_with_protection_cannot_be_assigned_to_other_blocked_hits() {
        with_detector(|d| {
            setup(d, 150);
            d.packet(&[
                hit(205, 150),
                event(
                    205,
                    "player_hurt",
                    json!({"user_id":62,"damage_amount":1,"health":149}),
                ),
            ]);
            confirm(d, 149, 200);
            assert!(d.pending.is_empty());
        });
        with_detector(|d| {
            setup(d, 20);
            d.packet(&[event(
                205,
                "player_hurt",
                json!({"user_id":62,"damage_amount":1,"health":19}),
            )]);
            confirm(d, 19, 30);
            assert!(d.pending.is_empty());
        });
    }
    #[test]
    fn routine_uber_and_missing_counter_are_not_saves() {
        with_detector(|d| {
            setup(d, 250);
            confirm(d, 250, 10);
            assert!(d.pending.is_empty());
        });
        for value in [Value::Null, json!(140), json!(-1)] {
            with_detector(|d| {
                setup(d, 20);
                let mut s = state(210, 20, 140);
                s["players"][0]["damage_blocked_total"] = value;
                d.state(&s);
                assert!(d.pending.is_empty());
            });
        }
    }
    #[test]
    fn unknown_counter_baseline_and_resets_do_not_create_proof() {
        with_detector(|d| {
            let mut s = state(200, 20, 140);
            s["players"][0]["damage_blocked_total"] = Value::Null;
            d.state(&s);
            d.packet(&deploy(201));
            d.packet(&[hit(202, 20)]);
            confirm(d, 20, 30);
            assert!(d.pending.is_empty());
        });
        with_detector(|d| {
            setup(d, 20);
            d.state(&state(205, 20, 0));
            d.state(&state(210, 20, 200));
            assert!(d.pending.is_empty());
        });
    }
    #[test]
    fn multiple_recipients_and_self_hits_make_aggregate_ambiguous() {
        for user in [78, 9] {
            with_detector(|d| {
                setup(d, 20);
                if user == 9 {
                    let mut s = state(203, 20, 140);
                    s["players"]
                        .as_array_mut()
                        .unwrap()
                        .push(player(9, 17, "red", 100));
                    d.state(&s);
                }
                d.packet(&[event(
                    204,
                    "player_hurt",
                    json!({"user_id":user,"attacker":72,"damage_amount":0,"health":100}),
                )]);
                confirm(d, 20, 500);
                assert!(d.pending.is_empty());
            });
        }
    }
    #[test]
    fn unknown_recipient_blocks_and_enemy_team_hit_does_not_contaminate() {
        with_detector(|d| {
            setup(d, 20);
            d.packet(&[event(
                204,
                "player_hurt",
                json!({"user_id":999,"damage_amount":0,"health":10}),
            )]);
            confirm(d, 20, 30);
            assert!(d.pending.is_empty());
        });
        with_detector(|d| {
            setup(d, 20);
            d.packet(&[event(
                204,
                "player_hurt",
                json!({"user_id":72,"damage_amount":0,"health":100}),
            )]);
            confirm(d, 20, 30);
            assert_eq!(d.pending.len(), 1);
        });
    }
    #[test]
    fn two_possible_medics_or_misleading_provider_are_rejected() {
        with_detector(|d| {
            let mut s = state(200, 20, 140);
            s["players"]
                .as_array_mut()
                .unwrap()
                .push(player(9, 17, "red", 150));
            d.state(&s);
            let mut events = deploy(201);
            events.push(event(201, "player_chargedeployed", json!({"user_id":9})));
            events.push(event(
                201,
                "player_invulned",
                json!({"user_id":62,"medic_user_id":9}),
            ));
            d.packet(&events);
            d.packet(&[hit(202, 20)]);
            confirm(d, 20, 30);
            assert!(d.pending.is_empty());
        });
        with_detector(|d| {
            d.state(&state(200, 20, 140));
            let mut events = deploy(201);
            events[1].event.insert("medic_user_id".into(), json!(7)); // entity != user
            d.packet(&events);
            d.packet(&[hit(202, 20)]);
            confirm(d, 20, 30);
            assert!(d.pending.is_empty());
        });
    }
    #[test]
    fn unknown_target_and_missing_protection_are_not_inferred() {
        for field in ["medigun_target", "invulnerable"] {
            with_detector(|d| {
                let mut s = state(200, 20, 140);
                if field == "medigun_target" {
                    s["players"][0][field] = Value::Null;
                } else {
                    s["players"][1][field] = Value::Null;
                }
                d.state(&s);
                d.packet(&deploy(201));
                d.packet(&[hit(202, 20)]);
                confirm(d, 20, 30);
                assert!(d.pending.is_empty());
            });
        }
    }
    #[test]
    fn target_switch_and_decoded_detach_allow_only_short_lingering_link() {
        for target in [json!({"entity_id":8}), json!({"entity_id":null})] {
            with_detector(|d| {
                setup(d, 20);
                let mut s = state(203, 20, 140);
                s["players"][0]["medigun_target"] = target;
                d.state(&s);
                d.packet(&[hit(204, 20)]);
                confirm(d, 20, 50);
                assert_eq!(d.pending.len(), 1);
            });
        }
        with_detector(|d| {
            setup(d, 20);
            let mut s = state(203, 20, 140);
            s["players"][0]["medigun_target"] = Value::Null;
            d.state(&s);
            d.packet(&[hit(204, 20)]);
            confirm(d, 20, 50);
            assert!(d.pending.is_empty());
        });
    }
    #[test]
    fn flash_recipient_must_have_named_link_and_target_history() {
        with_detector(|d| {
            d.state(&state(200, 20, 140));
            let mut events = deploy(201);
            events[1].event.insert("user_id".into(), json!(9));
            d.packet(&events);
            d.packet(&[
                event(
                    202,
                    "player_invulned",
                    json!({"user_id":62,"medic_user_id":78}),
                ),
                hit(202, 20),
            ]);
            confirm(d, 20, 30);
            assert_eq!(d.pending.len(), 1);
        });
    }
    #[test]
    fn unsupported_charge_types_do_not_start_deployment() {
        for kind in ["kritzkrieg", "quickfix", "vaccinator", ""] {
            with_detector(|d| {
                let mut s = state(200, 20, 140);
                s["players"][0]["medigun"] = json!(kind);
                d.state(&s);
                d.packet(&deploy(201));
                d.packet(&[hit(202, 20)]);
                confirm(d, 20, 30);
                assert!(d.pending.is_empty());
            });
        }
    }
    #[test]
    fn outcome_window_and_team_are_required() {
        with_detector(|d| {
            setup(d, 20);
            confirm(d, 20, 30);
            d.state(&state(1100, 20, 170));
            d.packet(&[kill(1101)]);
            assert!(output(d).is_empty());
        });
        with_detector(|d| {
            setup(d, 20);
            confirm(d, 20, 30);
            d.packet(&[event(211, "teamplay_point_captured", json!({"team":3}))]);
            assert!(output(d).is_empty());
        });
        for kind in ["teamplay_point_captured", "teamplay_round_win"] {
            with_detector(|d| {
                setup(d, 20);
                confirm(d, 20, 30);
                d.packet(&[event(211, kind, json!({"team":2}))]);
                assert_eq!(output(d).len(), 1);
            });
        }
    }
    #[test]
    fn outcome_before_delayed_stat_arrives_is_retained() {
        with_detector(|d| {
            setup(d, 20);
            d.packet(&[kill(205)]);
            confirm(d, 20, 30);
            assert_eq!(output(d).len(), 1);
        });
        with_detector(|d| {
            setup(d, 20);
            d.packet(&[event(201, "teamplay_point_captured", json!({"team":2}))]);
            confirm(d, 20, 30);
            assert!(output(d).is_empty());
        });
    }
    #[test]
    fn later_death_preserves_a_rescue_already_followed_by_a_kill() {
        with_detector(|d| {
            setup(d, 20);
            confirm(d, 20, 30);
            d.packet(&[kill(211)]);
            d.packet(&[event(
                212,
                "player_death",
                json!({"user_id":62,"attacker":9}),
            )]);
            assert_eq!(output(d).len(), 1);
        });
    }
    #[test]
    fn death_identity_pvs_and_round_breaks_prevent_pending_rescue() {
        for user in [62, 78] {
            for kind in [
                "player_death",
                "player_spawn",
                "player_disconnect",
                "player_changeclass",
                "player_team",
            ] {
                with_detector(|d| {
                    setup(d, 20);
                    d.packet(&[event(205, kind, json!({"user_id":user}))]);
                    confirm(d, 20, 30);
                    assert!(d.pending.is_empty());
                });
            }
        }
        for key in ["steam_id", "in_pvs", "life_state"] {
            with_detector(|d| {
                setup(d, 20);
                let mut s = state(205, 20, 140);
                s["players"][1][key] = match key {
                    "steam_id" => json!("new"),
                    "in_pvs" => json!(false),
                    _ => json!("dead"),
                };
                d.state(&s);
                confirm(d, 20, 30);
                assert!(d.pending.is_empty());
            });
        }
        with_detector(|d| {
            setup(d, 20);
            d.packet(&[event(205, "teamplay_round_start", json!({}))]);
            confirm(d, 20, 30);
            assert!(d.pending.is_empty());
        });
    }
    #[test]
    fn confirmed_rescue_remains_credited_if_medic_dies_before_followup() {
        with_detector(|d| {
            setup(d, 20);
            confirm(d, 20, 30);
            d.packet(&[event(
                211,
                "player_death",
                json!({"user_id":78,"attacker":72}),
            )]);
            d.packet(&[kill(212)]);
            assert_eq!(output(d).len(), 1);
        });
    }
    #[test]
    fn counter_reset_stops_new_outcomes_even_after_a_proof() {
        with_detector(|d| {
            setup(d, 20);
            confirm(d, 20, 30);
            d.state(&state(211, 20, 0));
            d.packet(&[kill(212)]);
            assert!(output(d).is_empty());
        });
    }
    #[test]
    fn entity_reuse_cannot_inherit_previous_target_or_counter() {
        with_detector(|d| {
            setup(d, 20);
            let mut s = state(205, 20, 140);
            s["players"][1]["user_id"] = json!(99);
            d.state(&s);
            confirm(d, 20, 30);
            assert!(d.pending.is_empty());
        });
    }
    #[test]
    fn delayed_counter_cannot_reassign_an_older_hit_to_a_recent_one() {
        with_detector(|d| {
            setup(d, 200);
            for tick in (210..=400).step_by(10) {
                d.state(&state(tick, 200, 140));
            }
            d.packet(&[hit(405, 200)]);
            d.state(&state(410, 200, 340));
            assert!(d.pending.is_empty());
        });
    }
    #[test]
    fn gaps_and_clock_regressions_fail_closed() {
        with_detector(|d| {
            setup(d, 20);
            d.state(&state(400, 20, 170));
            assert!(d.pending.is_empty());
        });
        with_detector(|d| {
            setup(d, 20);
            d.state(&state(199, 20, 170));
            assert!(d.pending.is_empty());
        });
        with_detector(|d| {
            setup(d, 20);
            let mut s = state(210, 20, 170);
            s["server_tick"] = Value::Null;
            d.state(&s);
            assert!(d.pending.is_empty());
        });
    }
    #[test]
    fn only_healer_pov_and_unambiguous_stock_scope_can_start() {
        let rounds = vec![round()];
        for user in [78, 62] {
            let ctx = DemoContext {
                analysis_scope: "pov_player_only".into(),
                pov_player_user_id: Some(user),
                ..DemoContext::default()
            };
            let mut d = Detector::new(100.0, &rounds, &ctx);
            setup(&mut d, 20);
            confirm(&mut d, 20, 30);
            assert_eq!(d.pending.len(), usize::from(user == 78));
        }
    }
    #[test]
    fn same_deployment_does_not_emit_duplicate_rescues() {
        with_detector(|d| {
            setup(d, 20);
            confirm(d, 20, 30);
            d.packet(&[hit(212, 20)]);
            d.state(&state(215, 20, 220));
            assert_eq!(d.pending.len(), 1);
        });
    }
    #[test]
    fn normal_analysis_exports_save_and_keeps_legacy_candidates_identical() {
        let root = std::env::temp_dir().join(format!(
            "uber-save-integration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut events = vec![
            event(100, "teamplay_round_start", json!({})),
            event(101, "teamplay_round_active", json!({})),
        ];
        events.extend(deploy(201));
        events.push(hit(202, 20));
        events.push(kill(211));
        events.push(event(2000, "teamplay_round_win", json!({"team":2})));
        fs::write(
            root.join("events.ndjson"),
            events
                .iter()
                .map(|e| serde_json::to_string(e).unwrap())
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        let states = vec![state(200, 20, 140), state(210, 20, 170)];
        fs::write(
            root.join("state_samples.ndjson"),
            states
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        fs::write(
            root.join("header.json"),
            r#"{"map":"cp_test","ticks":3000}"#,
        )
        .unwrap();
        fs::write(
            root.join("bookmarks.json"),
            r#"{"bookmarks":[{"tick":400,"comment":"keep me"}]}"#,
        )
        .unwrap();
        let manifest = json!({"source_demo":"source.dem","demo_capture":{"classification":"stv"}});
        fs::write(root.join("manifest.json"), manifest.to_string()).unwrap();
        let before = analyze_export(&root, None).unwrap();
        let mut enabled_manifest = manifest;
        enabled_manifest["server_tick_interval"] = json!(0.01);
        fs::write(root.join("manifest.json"), enabled_manifest.to_string()).unwrap();
        let enabled = analyze_export(&root, None).unwrap();
        let save = enabled
            .iter()
            .find(|c| c.primary_tag == "uber_save")
            .unwrap();
        assert_eq!(save.kill_count(), 0);
        assert_eq!(save.attacker_user_id, 78);
        assert_eq!(save.overall_score, SAVE_SCORE);
        assert_eq!(save.point_of_kill_ticks, vec![151, 152, 160, 161]);
        assert_eq!(save.metrics["patient_user_id"], json!(62));
        let remaining = enabled
            .iter()
            .filter(|c| c.primary_tag != "uber_save")
            .collect::<Vec<_>>();
        assert_eq!(
            serde_json::to_value(&remaining).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
        assert!(fs::read_to_string(root.join("frag_candidates.ndjson"))
            .unwrap()
            .contains("uber_save"));
        // Unknown legacy evidence never creates a save; it also does not
        // remove the existing frag or bookmark candidates.
        let legacy_states = states
            .iter()
            .cloned()
            .map(|mut s| {
                for p in s["players"].as_array_mut().unwrap() {
                    p.as_object_mut().unwrap().remove("damage_blocked_total");
                    p.as_object_mut().unwrap().remove("medigun_target");
                }
                s
            })
            .collect::<Vec<_>>();
        fs::write(
            root.join("state_samples.ndjson"),
            legacy_states
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        let legacy = analyze_export(&root, None).unwrap();
        assert_eq!(legacy.len(), before.len());
        assert!(legacy.iter().all(|c| c.primary_tag != "uber_save"));
        fs::write(
            root.join("state_samples.ndjson"),
            states
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        let ctx = DemoContext::default();
        let rounds = vec![round()];
        for (key, value) in [
            ("parser_reported_incomplete", json!(true)),
            ("server_tick_interval", Value::Null),
            ("server_tick_interval", json!(0)),
            ("server_tick_interval", json!(100)),
        ] {
            let mut invalid = enabled_manifest.clone();
            invalid[key] = value;
            let mut cs = vec![];
            append(
                &root,
                &events,
                &rounds,
                &ctx,
                &invalid,
                "source.dem",
                "cp_test",
                &mut cs,
                None,
                None,
            )
            .unwrap();
            assert!(cs.is_empty());
        }
        let mut cs = vec![];
        append(
            &root,
            &events,
            &rounds,
            &ctx,
            &enabled_manifest,
            "source.dem",
            "mvm_test",
            &mut cs,
            None,
            None,
        )
        .unwrap();
        assert!(cs.is_empty());
        let cancelled = AtomicBool::new(true);
        assert!(append(
            &root,
            &events,
            &rounds,
            &ctx,
            &enabled_manifest,
            "source.dem",
            "cp_test",
            &mut cs,
            Some(&cancelled),
            None
        )
        .is_err());
        // Imported candidates retain the complete additive evidence.
        let loaded: Candidate =
            serde_json::from_value(serde_json::to_value(save).unwrap()).unwrap();
        assert_eq!(loaded.metrics, save.metrics);
        fs::remove_dir_all(root).unwrap();
    }
}
