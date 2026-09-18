//! The door's pure core (constitution 1.5.1, Principle III): the v2 decision, unchanged, now run
//! as a gameplay system on a `DoorState` component. No `Gd`, no `NonSend`, no engine.

use bevy_ecs::prelude::*;

use crate::ecs::event::DoorBodyEntered;
use crate::ecs::markers::PlayOpen;

/// Replaces `open: bool` — a one-way flag, once `true`, never reset.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum DoorState {
    Closed,
    Open,
}

/// `v1`: `door.rs:26-29` — only a `Player` body on a currently-`Closed` door opens it
/// (returning the `bool` "play the open animation now"); anything else is a no-op.
pub fn on_body(state: DoorState, is_player: bool) -> (DoorState, bool) {
    match (state, is_player) {
        (DoorState::Closed, true) => (DoorState::Open, true),
        (state, _) => (state, false),
    }
}

/// `Phase::Gameplay` of the FIXED schedule (research.md R8): consumes the resolved
/// `DoorBodyEntered` messages and flags the entity with `PlayOpen` when the v2 decision says so;
/// `sync_out_door` consumes the flag in the same run.
pub fn open_on_player(
    mut reader: MessageReader<DoorBodyEntered>,
    mut doors: Query<&mut DoorState>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        // `Err`: the entity was despawned after the drain (validity sweep) — skip.
        let Ok(mut state) = doors.get_mut(msg.entity) else {
            continue;
        };
        let (next, play) = on_body(*state, msg.is_player);
        *state = next;
        if play {
            commands.entity(msg.entity).insert(PlayOpen);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::RunSystemOnce;
    use godot::obj::InstanceId;

    use super::*;
    use crate::ecs::apply::apply_non_register;
    use crate::ecs::event::InboundEvent;
    use crate::ecs::index::EntityIndex;

    /// research.md R9's recipe: a bare World with the resources the system and the drain need.
    fn world_with_door(state: DoorState) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(EntityIndex::default());
        world.insert_resource(Messages::<DoorBodyEntered>::default());
        let door = world.spawn(state).id();
        (world, door)
    }

    fn run(world: &mut World, door: Entity, is_player: bool) -> (DoorState, bool) {
        world
            .resource_mut::<Messages<DoorBodyEntered>>()
            .write(DoorBodyEntered { entity: door, is_player });
        world.run_system_once(open_on_player).unwrap();
        (*world.get::<DoorState>(door).unwrap(), world.get::<PlayOpen>(door).is_some())
    }

    // The three v2 cases (v2 door.rs:29-48), now as system tests.

    #[test]
    fn closed_door_opens_for_a_player() {
        let (mut world, door) = world_with_door(DoorState::Closed);
        assert_eq!(run(&mut world, door, true), (DoorState::Open, true));
    }

    #[test]
    fn closed_door_ignores_a_non_player_body() {
        let (mut world, door) = world_with_door(DoorState::Closed);
        assert_eq!(run(&mut world, door, false), (DoorState::Closed, false));
    }

    #[test]
    fn open_door_does_not_retrigger_for_a_player() {
        let (mut world, door) = world_with_door(DoorState::Open);
        assert_eq!(run(&mut world, door, true), (DoorState::Open, false));
    }

    // The round-trip through the drain (FR-016): id → entity → message → system.

    #[test]
    fn event_for_registered_id_reaches_its_entity() {
        let (mut world, door) = world_with_door(DoorState::Closed);
        let id = InstanceId::from_i64(7);
        world.resource_mut::<EntityIndex>().register_if_absent(id, || door);
        apply_non_register(&mut world, InboundEvent::DoorBodyEntered { id, is_player: true });
        world.run_system_once(open_on_player).unwrap();
        assert_eq!(*world.get::<DoorState>(door).unwrap(), DoorState::Open);
        assert!(world.get::<PlayOpen>(door).is_some());
    }

    #[test]
    fn event_for_unknown_id_is_dropped() {
        let (mut world, door) = world_with_door(DoorState::Closed);
        apply_non_register(
            &mut world,
            InboundEvent::DoorBodyEntered { id: InstanceId::from_i64(99), is_player: true },
        );
        assert!(world.resource::<Messages<DoorBodyEntered>>().is_empty());
        world.run_system_once(open_on_player).unwrap();
        assert_eq!(*world.get::<DoorState>(door).unwrap(), DoorState::Closed);
        assert!(world.get::<PlayOpen>(door).is_none());
    }
}
