use serde::Serialize;
use std::collections::HashMap;
use std::io::{self, Write};
use tf_demo_parser::demo::data::game_state::GameState;
use tf_demo_parser::demo::message::packetentities::{
    PacketEntitiesMessage, PacketEntity, UpdateType,
};
use tf_demo_parser::demo::message::Message;
use tf_demo_parser::demo::parser::ParserState;
use tf_demo_parser::demo::sendprop::{SendPropIdentifier, SendPropValue};

pub const FILE: &str = "sticky_evidence.ndjson";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum EntityKind {
    Player,
    Pipebomb,
}

#[derive(Clone, Copy, Serialize)]
struct Identity {
    entity_id: u32,
    serial: Option<u32>,
    entity_kind: EntityKind,
}

#[derive(Clone, Copy)]
struct Track {
    identity: Identity,
    user_id: Option<u16>,
    in_pvs: bool,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum EntityAction {
    Enter,
    Delta,
    Leave,
    Delete,
    Removed,
    Replaced,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum PropertySource {
    Explicit,
    Baseline,
}

#[derive(Serialize)]
struct Observation {
    field: &'static str,
    source: PropertySource,
    value: SendPropValue,
}

#[derive(Clone, Copy, Serialize)]
pub struct PacketPosition {
    pub packet_sequence: u64,
    pub demo_tick: u32,
    pub server_tick: Option<u32>,
}

#[derive(Serialize)]
struct EvidenceRow {
    #[serde(flatten)]
    position: PacketPosition,
    message_index: usize,
    #[serde(flatten)]
    record: EvidenceRecord,
}

#[derive(Serialize)]
#[serde(tag = "record_type", rename_all = "snake_case")]
enum EvidenceRecord {
    Entity {
        #[serde(flatten)]
        identity: Identity,
        entity_order: usize,
        action: EntityAction,
        in_pvs: bool,
        user_id: Option<u16>,
        observations: Vec<Observation>,
    },
    GameEvent {
        event_index_in_packet: usize,
        event_type: String,
    },
}

#[derive(Default)]
pub struct StickyEvidence {
    entities: HashMap<u32, Track>,
    temporary_messages: usize,
}

impl StickyEvidence {
    pub fn start_packet(&mut self) {
        self.temporary_messages = 0;
    }

    pub fn observe(
        &mut self,
        position: PacketPosition,
        message_index: usize,
        message: &Message<'_>,
        parser: &ParserState,
        game: &GameState,
        out: &mut impl Write,
    ) -> io::Result<()> {
        let mut write = |record| {
            serde_json::to_writer(
                &mut *out,
                &EvidenceRow {
                    position,
                    message_index,
                    record,
                },
            )?;
            out.write_all(b"\n")
        };
        match message {
            Message::TempEntities(_) => self.temporary_messages += 1,
            Message::PacketEntities(message) => {
                self.observe_entities(message, parser, game, write)?
            }
            Message::GameEvent(event) => {
                let event_type = event.event_type.as_str();
                let round_start = matches!(event_type, "round_start" | "teamplay_round_start");
                if round_start {
                    self.entities
                        .retain(|_, track| track.identity.entity_kind == EntityKind::Player);
                }
                if round_start
                    || matches!(
                        event_type,
                        "player_death"
                            | "demoman_det_stickies"
                            | "player_destroyed_pipebomb"
                            | "object_deflected"
                            | "teamplay_round_win"
                            | "teamplay_round_active"
                    )
                {
                    write(EvidenceRecord::GameEvent {
                        event_index_in_packet: message_index - self.temporary_messages,
                        event_type: event_type.to_owned(),
                    })?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn observe_entities(
        &mut self,
        message: &PacketEntitiesMessage,
        parser: &ParserState,
        game: &GameState,
        mut write: impl FnMut(EvidenceRecord) -> io::Result<()>,
    ) -> io::Result<()> {
        for (entity_order, entity) in message.entities.iter().enumerate() {
            if let Some(record) = self.observe_entity(entity, entity_order, parser, game) {
                write(record)?;
            }
        }
        for (index, id) in message.removed_entities.iter().enumerate() {
            if let Some(track) = self.entities.remove(&u32::from(*id)) {
                write(boundary(
                    track,
                    message.entities.len() + index,
                    EntityAction::Removed,
                ))?;
            }
        }
        Ok(())
    }

    fn observe_entity(
        &mut self,
        entity: &PacketEntity,
        entity_order: usize,
        parser: &ParserState,
        game: &GameState,
    ) -> Option<EvidenceRecord> {
        let id = u32::from(entity.entity_index);
        // Leave and Delete may present a placeholder class. Only a tracked
        // index can connect the operation to a previously observed entity.
        match entity.update_type {
            UpdateType::Leave => {
                let track = self.entities.get_mut(&id)?;
                track.in_pvs = false;
                return Some(boundary(*track, entity_order, EntityAction::Leave));
            }
            UpdateType::Delete => {
                let track = self.entities.remove(&id)?;
                return Some(boundary(track, entity_order, EntityAction::Delete));
            }
            UpdateType::Delta | UpdateType::Enter => {}
        }

        let previous = self.entities.get(&id).copied();
        let entering = entity.update_type == UpdateType::Enter;
        let entity_kind = match parser
            .server_classes
            .get(usize::from(entity.server_class))
            .map(|class| class.name.as_str())
        {
            Some("CTFPlayer") => EntityKind::Player,
            Some("CTFGrenadePipebombProjectile") => EntityKind::Pipebomb,
            _ if entering => {
                return self
                    .entities
                    .remove(&id)
                    .map(|track| boundary(track, entity_order, EntityAction::Replaced));
            }
            _ => return None,
        };
        let serial = if entering {
            Some(entity.serial_number)
        } else {
            previous
                .filter(|track| track.identity.entity_kind == entity_kind)
                .and_then(|track| track.identity.serial)
        };
        let identity = Identity {
            entity_id: id,
            serial,
            entity_kind,
        };
        let user_id = if entity_kind == EntityKind::Player {
            game.players
                .iter()
                .find(|player| player.entity == entity.entity_index)
                .and_then(|player| player.info.as_ref())
                .map(|info| u16::from(info.user_id))
        } else {
            None
        };
        let observations = observed_props(entity, entity_kind, parser);
        self.entities.insert(
            id,
            Track {
                identity,
                user_id,
                in_pvs: entity.in_pvs,
            },
        );
        if !entering
            && observations.is_empty()
            && previous.is_some_and(|track| {
                track.user_id == user_id
                    && track.identity.entity_kind == entity_kind
                    && track.in_pvs == entity.in_pvs
            })
        {
            return None;
        }
        Some(EvidenceRecord::Entity {
            identity,
            entity_order,
            action: if entering {
                EntityAction::Enter
            } else {
                EntityAction::Delta
            },
            in_pvs: entity.in_pvs,
            user_id,
            observations,
        })
    }
}

fn boundary(track: Track, entity_order: usize, action: EntityAction) -> EvidenceRecord {
    EvidenceRecord::Entity {
        identity: track.identity,
        entity_order,
        action,
        in_pvs: false,
        user_id: track.user_id,
        observations: Vec::new(),
    }
}

fn observed_props(
    entity: &PacketEntity,
    kind: EntityKind,
    parser: &ParserState,
) -> Vec<Observation> {
    let fields = match kind {
        EntityKind::Pipebomb => PIPE_FIELDS,
        EntityKind::Player => PLAYER_FIELDS,
    };
    // On Enter, props() yields explicit values ahead of applicable baseline
    // values. On Delta, an absent field means this message did not update it.
    entity
        .props(parser)
        .filter_map(|prop| {
            let field = fields.iter().find(|(id, _)| *id == prop.identifier)?.1;
            let explicit = entity
                .props
                .iter()
                .any(|raw| raw.identifier == prop.identifier);
            let source = if explicit {
                PropertySource::Explicit
            } else {
                PropertySource::Baseline
            };
            Some(Observation {
                field,
                source,
                value: prop.value,
            })
        })
        .collect()
}

const PIPE_FIELDS: &[(SendPropIdentifier, &str)] = &[
    (
        SendPropIdentifier::new("DT_TFProjectile_Pipebomb", "m_iType"),
        "pipe_mode",
    ),
    (
        SendPropIdentifier::new("DT_TFProjectile_Pipebomb", "m_bTouched"),
        "touched",
    ),
    (
        SendPropIdentifier::new("DT_TFProjectile_Pipebomb", "m_hLauncher"),
        "launcher_handle",
    ),
    (
        SendPropIdentifier::new("DT_TFProjectile_Pipebomb", "m_bDefensiveBomb"),
        "defensive_bomb",
    ),
    (
        SendPropIdentifier::new("DT_BaseProjectile", "m_hOriginalLauncher"),
        "original_launcher_handle",
    ),
    (
        SendPropIdentifier::new("DT_BaseGrenade", "m_hThrower"),
        "thrower_handle",
    ),
    (
        SendPropIdentifier::new("DT_BaseEntity", "m_hOwnerEntity"),
        "owner_handle",
    ),
    (
        SendPropIdentifier::new("DT_BaseEntity", "m_iTeamNum"),
        "team",
    ),
    (
        SendPropIdentifier::new("DT_TFWeaponBaseGrenadeProj", "m_hDeflectOwner"),
        "deflect_owner_handle",
    ),
    (
        SendPropIdentifier::new("DT_TFWeaponBaseGrenadeProj", "m_iDeflected"),
        "deflection_count",
    ),
    (
        SendPropIdentifier::new("DT_TFWeaponBaseGrenadeProj", "m_vecOrigin"),
        "position",
    ),
];

const PLAYER_FIELDS: &[(SendPropIdentifier, &str)] = &[
    (
        SendPropIdentifier::new("m_hMyWeapons", "000"),
        "primary_handle",
    ),
    (
        SendPropIdentifier::new("m_hMyWeapons", "001"),
        "secondary_handle",
    ),
    (
        SendPropIdentifier::new("m_hMyWeapons", "002"),
        "melee_handle",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use tf_demo_parser::demo::message::gameevent::GameEventMessage;
    use tf_demo_parser::demo::message::packetentities::BaselineIndex;
    use tf_demo_parser::demo::message::tempentities::TempEntitiesMessage;
    use tf_demo_parser::demo::packet::datatable::{SendTable, ServerClass};
    use tf_demo_parser::demo::sendprop::SendProp;

    fn parser() -> ParserState {
        let mut parser = ParserState::new(24, |_| true, false);
        for (id, name, table) in [
            (
                0u16,
                "CTFGrenadePipebombProjectile",
                "DT_TFProjectile_Pipebomb",
            ),
            (1, "CTFPlayer", "DT_TFPlayer"),
        ] {
            parser.server_classes.push(ServerClass {
                id: id.into(),
                name: name.into(),
                data_table: table.into(),
            });
            parser.send_tables.push(SendTable {
                name: table.into(),
                needs_decoder: false,
                flattened_props: Vec::new(),
            });
        }
        parser
    }

    fn entity(action: UpdateType, serial: u32, props: Vec<SendProp>) -> PacketEntity {
        PacketEntity {
            server_class: 0u16.into(),
            entity_index: 42u32.into(),
            props,
            in_pvs: matches!(action, UpdateType::Enter | UpdateType::Delta),
            update_type: action,
            serial_number: serial,
            delay: None,
            delta: None,
            baseline_index: BaselineIndex::First,
        }
    }

    fn prop(identifier: SendPropIdentifier, value: i64) -> SendProp {
        SendProp {
            index: 0,
            identifier,
            value: SendPropValue::Integer(value),
        }
    }

    fn entities(entities: Vec<PacketEntity>) -> Message<'static> {
        Message::PacketEntities(PacketEntitiesMessage {
            entities,
            ..Default::default()
        })
    }

    fn observations(messages: Vec<Message<'_>>, parser: &ParserState) -> Vec<Value> {
        let mut tracker = StickyEvidence::default();
        let mut output = Vec::new();
        let position = PacketPosition {
            packet_sequence: 6,
            demo_tick: 15,
            server_tick: Some(500),
        };
        for (index, message) in messages.iter().enumerate() {
            tracker
                .observe(
                    position,
                    index,
                    message,
                    parser,
                    &GameState::default(),
                    &mut output,
                )
                .unwrap();
        }
        output
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect()
    }

    #[test]
    fn enter_retains_baseline_and_explicit_provenance() {
        let mut parser = parser();
        let mut baseline = PacketEntitiesMessage {
            updated_base_line: true,
            ..Default::default()
        };
        baseline.entities.push(entity(
            UpdateType::Enter,
            99,
            vec![prop(PIPE_FIELDS[0].0, 2), prop(PIPE_FIELDS[1].0, 1)],
        ));
        parser.handle_message(Message::PacketEntities(baseline), 0u32.into());
        let mut entry = entity(UpdateType::Enter, 0, vec![prop(PIPE_FIELDS[1].0, 0)]);
        entry.delta = Some(1u32.into());
        entry.baseline_index = BaselineIndex::Second;
        let rows = observations(vec![entities(vec![entry])], &parser);
        assert_eq!(rows[0]["serial"], 0);
        assert_eq!(
            rows[0]["observations"],
            json!([
                {"field":"touched","source":"explicit","value":0},
                {"field":"pipe_mode","source":"baseline","value":2},
            ])
        );
        assert!(rows[0]["user_id"].is_null());
    }

    #[test]
    fn records_visibility_removal_and_reuse_in_message_order() {
        let parser = parser();
        let removal = Message::PacketEntities(PacketEntitiesMessage {
            removed_entities: vec![42u32.into()],
            ..Default::default()
        });
        let mut player = entity(
            UpdateType::Enter,
            77,
            vec![prop(PLAYER_FIELDS[1].0, 163_885)],
        );
        player.entity_index = 3u32.into();
        player.server_class = 1u16.into();
        let rows = observations(
            vec![
                entities(vec![
                    entity(UpdateType::Enter, 10, vec![prop(PIPE_FIELDS[1].0, 1)]),
                    player,
                ]),
                entities(vec![entity(UpdateType::Leave, 0, vec![])]),
                entities(vec![entity(UpdateType::Enter, 10, vec![])]),
                entities(vec![entity(
                    UpdateType::Delta,
                    0,
                    vec![prop(PIPE_FIELDS[1].0, 0)],
                )]),
                removal,
                entities(vec![entity(
                    UpdateType::Delta,
                    0,
                    vec![prop(PIPE_FIELDS[1].0, 1)],
                )]),
                entities(vec![entity(UpdateType::Enter, 11, vec![])]),
                entities(vec![entity(UpdateType::Delete, 0, vec![])]),
            ],
            &parser,
        );
        assert_eq!(rows[0]["serial"], 10);
        assert_eq!(rows[1]["entity_kind"], "player");
        assert_eq!(rows[1]["observations"][0]["value"], 163_885);
        assert_eq!(rows[2]["action"], "leave");
        assert_eq!(rows[2]["serial"], 10);
        assert_eq!(rows[2]["in_pvs"], false);
        assert_eq!(rows[3]["serial"], 10);
        assert_eq!(rows[4]["observations"][0]["value"], 0);
        assert_eq!(rows[5]["action"], "removed");
        assert!(rows[6]["serial"].is_null());
        assert_eq!(rows[6]["observations"][0]["value"], 1);
        assert_eq!(rows[7]["serial"], 11);
        assert_eq!(rows[8]["action"], "delete");
        assert_eq!(rows[8]["serial"], 11);
    }

    #[test]
    fn round_boundary_clears_pipe_identity_and_keeps_legacy_event_index() {
        let event: GameEventMessage = serde_json::from_value(json!({
            "event_type_id": 1, "event_type": "teamplay_round_start",
            "event": {"type":"TeamPlayRoundStart","full_reset":true}
        }))
        .unwrap();
        let rows = observations(
            vec![
                entities(vec![entity(UpdateType::Enter, 10, vec![])]),
                Message::TempEntities(TempEntitiesMessage { events: Vec::new() }),
                Message::GameEvent(event),
                entities(vec![entity(
                    UpdateType::Delta,
                    0,
                    vec![prop(PIPE_FIELDS[1].0, 1)],
                )]),
            ],
            &parser(),
        );
        assert_eq!(rows[1]["record_type"], "game_event");
        assert_eq!(rows[1]["message_index"], 2);
        assert_eq!(rows[1]["event_index_in_packet"], 1);
        assert!(rows[2]["serial"].is_null());
    }
}
