use super::{check_runtime, Candidate, EventRecord, RuntimeGovernor};
use crate::models::PLANTED_STICKY_KILL;
use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::atomic::AtomicBool,
};

const SIDECAR: &str = "sticky_evidence.ndjson";
const MINIMUM_DWELL_NS: i128 = 2_000_000_000;
const HANDLE_INDEX_MASK: i64 = 2047;
const INVALID_HANDLE: i64 = 2_097_151;
const WEAPON_SLOTS: [&str; 3] = ["primary_handle", "secondary_handle", "melee_handle"];
const DWELL_BREAKS: [&str; 7] = [
    "position",
    "thrower_handle",
    "owner_handle",
    "launcher_handle",
    "original_launcher_handle",
    "deflect_owner_handle",
    "deflection_count",
];

type EventKey = (i64, i64);

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EntityKind {
    Pipebomb,
    Player,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Enter,
    Delta,
    Leave,
    Delete,
    Removed,
    Replaced,
}

#[derive(Deserialize)]
struct Observation {
    field: String,
    value: Value,
}

#[derive(Deserialize)]
#[serde(tag = "record_type", rename_all = "snake_case")]
enum EvidenceRow {
    Entity {
        entity_id: i64,
        entity_kind: EntityKind,
        serial: Option<i64>,
        action: Action,
        in_pvs: bool,
        user_id: Option<i64>,
        server_tick: Option<i64>,
        observations: Vec<Observation>,
    },
    GameEvent {
        packet_sequence: i64,
        event_index_in_packet: i64,
        event_type: String,
        server_tick: Option<i64>,
        demo_tick: i64,
    },
}

struct Entity {
    kind: EntityKind,
    serial: Option<i64>,
    user: Option<i64>,
    fields: HashMap<String, Value>,
    planted_since: Option<i64>,
}

impl Entity {
    fn int(&self, field: &str) -> Option<i64> {
        self.fields.get(field).and_then(Value::as_i64)
    }
}

fn valid_handle(handle: i64) -> bool {
    (1..INVALID_HANDLE).contains(&handle)
}

fn qualifies(active: &HashMap<i64, Entity>, event: &EventRecord, interval_ns: i64) -> bool {
    let Some(bomb) = active.get(&event.int(&["inflictor_ent_index"])) else {
        return false;
    };
    if bomb.kind != EntityKind::Pipebomb
        || bomb.serial.is_none()
        || bomb.int("pipe_mode") != Some(1)
    {
        return false;
    }
    let Some(ticks) = event
        .server_tick
        .zip(bomb.planted_since)
        .and_then(|(death, planted)| death.checked_sub(planted))
    else {
        return false;
    };
    if i128::from(ticks) * i128::from(interval_ns) < MINIMUM_DWELL_NS {
        return false;
    }
    let actor = event.int(&["attacker"]);
    let Some(thrower) = bomb
        .int("thrower_handle")
        .filter(|handle| valid_handle(*handle))
    else {
        return false;
    };
    let Some(launcher) = bomb
        .int("original_launcher_handle")
        .filter(|handle| valid_handle(*handle))
    else {
        return false;
    };
    if bomb.int("launcher_handle") != Some(launcher)
        || !active.get(&(thrower & HANDLE_INDEX_MASK)).is_some_and(|p| {
            p.kind == EntityKind::Player && p.serial == Some(thrower >> 11) && p.user == Some(actor)
        })
    {
        return false;
    }
    let mut inventories = active.values().filter(|p| {
        p.kind == EntityKind::Player
            && WEAPON_SLOTS
                .iter()
                .any(|slot| p.int(slot) == Some(launcher))
    });
    inventories.next().is_some_and(|p| p.user == Some(actor)) && inventories.next().is_none()
}

fn apply_row(
    row: EvidenceRow,
    active: &mut HashMap<i64, Entity>,
    deaths: &HashMap<EventKey, &EventRecord>,
    interval_ns: i64,
    eligible: &mut HashSet<EventKey>,
) {
    match row {
        EvidenceRow::Entity {
            entity_id,
            entity_kind,
            serial,
            action,
            in_pvs,
            user_id,
            server_tick,
            observations,
        } => {
            if !in_pvs || !matches!(action, Action::Enter | Action::Delta) {
                active.remove(&entity_id);
                return;
            }
            let entity = active.entry(entity_id).or_insert_with(|| Entity {
                kind: entity_kind,
                serial,
                user: user_id,
                fields: HashMap::new(),
                planted_since: None,
            });
            if matches!(action, Action::Enter)
                || entity.kind != entity_kind
                || entity.serial != serial
            {
                entity.fields.clear();
                entity.planted_since = None;
            }
            entity.kind = entity_kind;
            entity.serial = serial;
            entity.user = user_id;
            let mut touched = None;
            for Observation { field, value } in observations {
                if field == "touched" {
                    touched = Some(value.as_i64() == Some(1));
                } else if DWELL_BREAKS.contains(&field.as_str())
                    && entity
                        .fields
                        .get(&field)
                        .is_some_and(|prior| prior != &value)
                {
                    entity.planted_since = None;
                }
                entity.fields.insert(field, value);
            }
            if touched == Some(false) {
                entity.planted_since = None;
            } else if touched == Some(true)
                && entity.planted_since.is_none()
                && entity.fields.get("position").is_some_and(Value::is_object)
                && entity
                    .int("original_launcher_handle")
                    .is_some_and(valid_handle)
                && entity.int("thrower_handle").is_some_and(valid_handle)
            {
                entity.planted_since = server_tick.filter(|tick| *tick > 0);
            }
        }
        EvidenceRow::GameEvent {
            packet_sequence,
            event_index_in_packet,
            event_type,
            server_tick,
            demo_tick,
        } => {
            if matches!(event_type.as_str(), "round_start" | "teamplay_round_start") {
                active.retain(|_, entity| entity.kind == EntityKind::Player);
            } else if event_type == "player_death" {
                let key = (packet_sequence, event_index_in_packet);
                if deaths.get(&key).is_some_and(|event| {
                    event.server_tick == server_tick
                        && event.demo_tick == demo_tick
                        && qualifies(active, event, interval_ns)
                }) {
                    eligible.insert(key);
                }
            }
        }
    }
}

fn kill_key(kill: &Value) -> Option<EventKey> {
    Some((
        kill.get("packet_sequence")?.as_i64()?,
        kill.get("event_index_in_packet")?.as_i64()?,
    ))
}

fn add_tag(tags: &mut Vec<String>) {
    if !tags.iter().any(|tag| tag == PLANTED_STICKY_KILL) {
        tags.push(PLANTED_STICKY_KILL.into());
    }
}

fn apply_tags(
    candidates: &mut [Candidate],
    deaths: &HashMap<EventKey, &EventRecord>,
    eligible: &HashSet<EventKey>,
) {
    for candidate in candidates {
        let ticks: HashSet<i64> = candidate
            .kills
            .iter()
            .filter_map(|kill| {
                let key = kill_key(kill)?;
                if !eligible.contains(&key) {
                    return None;
                }
                let event = deaths.get(&key)?;
                let tick = kill.get("demo_tick")?.as_i64()?;
                (tick == event.demo_tick
                    && kill.get("attacker_user_id")?.as_i64()? == event.int(&["attacker"])
                    && kill.get("victim_user_id")?.as_i64()? == event.int(&["user_id", "userid"]))
                .then_some(tick)
            })
            .collect();
        if !ticks.is_empty() {
            add_tag(&mut candidate.tags);
            for group in &mut candidate.tick_tags {
                if ticks.contains(&group.demo_tick) {
                    add_tag(&mut group.tags);
                }
            }
        }
    }
}

pub(super) fn annotate(
    export: &Path,
    manifest: &Value,
    events: &[EventRecord],
    candidates: &mut [Candidate],
    cancelled: Option<&AtomicBool>,
    governor: Option<&RuntimeGovernor>,
) -> Result<()> {
    if manifest["sticky_evidence_format_version"].as_u64() != Some(1)
        || manifest["files"]["sticky_evidence"].as_str() != Some(SIDECAR)
    {
        return Ok(());
    }
    let Some(interval) = manifest["server_tick_interval"]
        .as_f64()
        .filter(|value| value.is_finite() && *value > 0.0)
    else {
        return Ok(());
    };
    let interval_ns = (interval * 1_000_000_000.0).round() as i64;
    let keys: HashSet<EventKey> = candidates
        .iter()
        .flat_map(|candidate| candidate.kills.iter().filter_map(kill_key))
        .collect();
    let deaths = events
        .iter()
        .filter_map(|event| {
            let actor = event.int(&["attacker"]);
            let victim = event.int(&["user_id", "userid"]);
            let key = (event.packet_sequence, event.event_index_in_packet);
            (event.event_type == "player_death"
                && actor > 0
                && victim > 0
                && actor != victim
                && event.event.get("weapon").and_then(Value::as_str)
                    == Some("tf_projectile_pipe_remote")
                && keys.contains(&key))
            .then_some((key, event))
        })
        .collect::<HashMap<_, _>>();
    if deaths.is_empty() {
        return Ok(());
    }
    // Evidence is optional. Commit annotations only after a successful scan.
    let Ok(input) = File::open(export.join(SIDECAR)) else {
        return Ok(());
    };
    let mut active = HashMap::new();
    let mut eligible = HashSet::new();
    for (index, line) in BufReader::new(input).lines().enumerate() {
        if index % 256 == 0 {
            check_runtime(cancelled, governor)?;
        }
        let Ok(line) = line else { return Ok(()) };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str(&line) else {
            return Ok(());
        };
        apply_row(row, &mut active, &deaths, interval_ns, &mut eligible);
    }
    check_runtime(cancelled, governor)?;
    apply_tags(candidates, &deaths, &eligible);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TickTagGroup;
    use serde_json::json;
    use std::{fs, time::SystemTime};

    #[test]
    fn tag_is_additive_and_requires_two_seconds_of_continuous_attributed_dwell() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("sticky-tag-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let actor = json!({"record_type":"entity","entity_id":12,"entity_kind":"player","serial":3,
            "action":"enter","in_pvs":true,"user_id":80,"server_tick":100,
            "observations":[{"field":"secondary_handle","value":605318}]});
        let bomb = json!({"record_type":"entity","entity_id":633,"entity_kind":"pipebomb","serial":83,
            "action":"enter","in_pvs":true,"server_tick":100,
            "observations":[{"field":"pipe_mode","value":1},{"field":"touched","value":1},
                {"field":"position","value":{"x":10,"y":20,"z":30}},
                {"field":"thrower_handle","value":6156},{"field":"launcher_handle","value":605318},
                {"field":"original_launcher_handle","value":605318}]});
        let break_dwell = json!({"record_type":"entity","entity_id":633,"entity_kind":"pipebomb","serial":83,
            "action":"delta","in_pvs":true,"server_tick":200,
            "observations":[{"field":"position","value":{"x":11,"y":20,"z":30}}]});
        let manifest = json!({"sticky_evidence_format_version":1,"files":{"sticky_evidence":SIDECAR},
            "server_tick_interval":0.014999999664723873});
        let legacy = json!({});
        for (tick, broken, mode, bad_owner, present, malformed, expected) in [
            (234, false, json!(1), false, true, false, true),
            (233, false, json!(1), false, true, false, false),
            (234, true, json!(1), false, true, false, false),
            (234, false, Value::Null, false, true, false, false),
            (234, false, json!(1), true, true, false, false),
            (234, false, json!(1), false, false, false, false),
            (234, false, json!(1), false, true, true, false),
        ] {
            let mut projectile = bomb.clone();
            projectile["observations"][0]["value"] = mode;
            let mut player = actor.clone();
            if bad_owner {
                player["serial"] = json!(4);
            }
            let mut rows = vec![player, projectile];
            if broken {
                rows.push(break_dwell.clone());
            }
            rows.push(
                json!({"record_type":"game_event","event_type":"player_death",
                "packet_sequence":20,"event_index_in_packet":4,"demo_tick":150,"server_tick":tick}),
            );
            let mut sidecar = rows
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n");
            if malformed {
                sidecar.push_str("\n{");
            }
            fs::write(root.join(SIDECAR), sidecar).unwrap();
            let event: EventRecord = serde_json::from_value(json!({"event_type":"player_death",
                "packet_sequence":20,"event_index_in_packet":4,"demo_tick":150,"server_tick":tick,
                "event":{"weapon":"tf_projectile_pipe_remote","attacker":80,"user_id":70,"inflictor_ent_index":633}})).unwrap();
            let before = Candidate {
                candidate_id: "r1-p80-t234".into(),
                clip_start_tick: 100,
                clip_end_tick: 250,
                overall_score: 48.,
                primary_tag: "other".into(),
                tick_tags: vec![TickTagGroup {
                    demo_tick: 150,
                    ..TickTagGroup::default()
                }],
                kills: vec![
                    json!({"packet_sequence":20,"event_index_in_packet":4,"demo_tick":150,
                    "attacker_user_id":80,"victim_user_id":70}),
                ],
                ..Candidate::default()
            };
            let mut actual = [before.clone()];
            annotate(
                &root,
                if present { &manifest } else { &legacy },
                &[event],
                &mut actual,
                None,
                None,
            )
            .unwrap();
            let mut after = before;
            if expected {
                after.tags.push(PLANTED_STICKY_KILL.into());
                after.tick_tags[0].tags.push(PLANTED_STICKY_KILL.into());
            }
            assert_eq!(actual[0], after);
            assert_eq!(actual[0].inferred_primary_tag(), "other");
        }
        fs::remove_dir_all(root).unwrap();
    }
}
