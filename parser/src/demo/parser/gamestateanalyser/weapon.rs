use crate::demo::data::attributes::{has_attribute, Attribute};
use crate::demo::data::game_state::{GameState, Handle, MedigunType, PlayerClassData};
use crate::demo::message::{EntityId, PacketEntity, UpdateType};
use crate::demo::sendprop::SendPropIdentifier;
use std::collections::HashMap;

pub fn handle_medigun_entity(
    state: &mut GameState,
    entity: &PacketEntity,
    outer_map_rev: &HashMap<EntityId, Handle>,
) {
    const OUTER: SendPropIdentifier = SendPropIdentifier::new("DT_AttributeContainer", "m_hOuter");
    const TARGET: SendPropIdentifier =
        SendPropIdentifier::new("DT_WeaponMedigun", "m_hHealingTarget");

    if entity.update_type == UpdateType::Enter {
        let mut ty = MedigunType::Uber;
        if has_attribute(&entity.props, Attribute::MedigunChargeIsCritBoost) {
            ty = MedigunType::Kritzkrieg;
        }
        if has_attribute(&entity.props, Attribute::MedigunChargeIsMegaHeal) {
            ty = MedigunType::Quickfix;
        }
        if has_attribute(&entity.props, Attribute::MedigunChargeIsResists) {
            ty = MedigunType::Vaccinator;
        }

        if let Some(handle) = entity.get_own_prop_value_by_identifier::<Handle>(OUTER) {
            if let Some(player) = state.get_player_by_weapon_handle(handle) {
                if let PlayerClassData::Medic { medigun, .. } = &mut player.class_data {
                    *medigun = ty;
                    player.medigun_target_observed = None;
                }
            }
        }
    }

    if entity.props.iter().any(|prop| prop.identifier == TARGET
        && Handle::try_from(&prop.value).is_err()) {
        if let Some(medic) = outer_map_rev.get(&entity.entity_index).copied()
            .and_then(|handle| state.get_player_by_weapon_handle(handle)) {
            medic.medigun_target_observed = None;
        }
    }

    if let Some(target_handle) = entity.get_own_prop_value_by_identifier::<Handle>(TARGET) {
        let target_id = state
            .get_player_by_handle(target_handle)
            .map(|target| target.entity);
        let medic = outer_map_rev
            .get(&entity.entity_index)
            .copied()
            .and_then(|self_handle| state.get_player_by_weapon_handle(self_handle));

        if let Some(medic) = medic {
            // 21-bit network EHANDLE sentinel, or sign-extended full sentinel.
            // An unresolved valid handle remains unknown, not a decoded detach.
            medic.medigun_target_observed = if matches!(target_handle.0, -1 | 2097151 | 4294967295) {
                Some(None)
            } else if target_handle.0 == 0 {
                // World/null handles must not match an unobserved player's
                // default handle zero in the legacy lookup.
                None
            } else {
                target_id.map(Some)
            };
            if let PlayerClassData::Medic {
                target,
                last_target,
                ..
            } = &mut medic.class_data
            {
                *target = target_id;
                if target_id.is_some() {
                    *last_target = target_id;
                }
            }
        }
    }
}

#[cfg(test)]
mod uber_target_tests {
    use super::*;
    use crate::demo::data::game_state::{Player, PlayerState};
    use crate::demo::message::packetentities::BaselineIndex;
    use crate::demo::packet::datatable::ClassId;
    use crate::demo::sendprop::{SendProp, SendPropValue};

    #[test]
    fn target_handles_distinguish_resolved_detached_unresolved_and_invalid() {
        let mut state = GameState::default();
        let mut medic = Player::default();
        medic.entity = 7u32.into();
        medic.weapons[1] = Handle(100);
        medic.in_pvs = true;
        medic.state = PlayerState::Alive;
        medic.class_data = PlayerClassData::default_for_class(crate::demo::parser::analyser::Class::Medic);
        let mut patient = Player::default();
        patient.entity = 6u32.into();
        patient.handle = Handle(983046);
        state.players = vec![medic, patient];
        let map = HashMap::from([(100u32.into(), Handle(100))]);
        for (raw, expected) in [(983046,Some(Some(6u32.into()))),
            (2097151,Some(None)),(-1,Some(None)),(4294967295,Some(None)),(123456,None),(0,None)] {
            let mut entity = PacketEntity {
                server_class: ClassId::from(0), entity_index: 100u32.into(),
                props: vec![SendProp { index:0, identifier:SendPropIdentifier::new("DT_WeaponMedigun", "m_hHealingTarget"),
                    value:SendPropValue::Integer(raw) }],
                in_pvs:true,update_type:UpdateType::Delta,serial_number:0,
                delay:None,delta:None,baseline_index:BaselineIndex::First,
            };
            handle_medigun_entity(&mut state,&entity,&map);
            assert_eq!(state.players[0].observed_medigun_target(),expected);
            // A malformed observation must not keep the last valid target.
            entity.props[0].value = SendPropValue::Vector(crate::demo::vector::Vector::default());
            handle_medigun_entity(&mut state,&entity,&map);
            assert_eq!(state.players[0].observed_medigun_target(),None);
        }
    }
}
