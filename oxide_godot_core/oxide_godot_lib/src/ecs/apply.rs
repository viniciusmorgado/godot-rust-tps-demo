//! Sync-layer application of the non-registration events (research.md R3): pure in the
//! constitution's sense — `&mut World`, no `Gd`, no engine — and tested without Godot.

use bevy_ecs::prelude::{Messages, World};

use super::event::{DoorBodyEntered, InboundEvent};
use super::index::EntityIndex;
use super::markers::Remove;
use super::NodeHandles;

/// Applies `Unregister`, `DoorBodyEntered` and `BlastAnimationFinished` directly to the World.
/// `Register` needs the engine handles and is routed by the driver to `ecs::apply_register`.
pub fn apply_non_register(world: &mut World, event: InboundEvent) {
    match event {
        InboundEvent::Register { .. } => {
            debug_assert!(false, "Register is routed to apply_register by the driver");
        }
        InboundEvent::Unregister { id } => {
            let removed = world.resource_mut::<EntityIndex>().unregister(id);
            if let Some(e) = removed {
                // The `Option`-returning accessor: tests need not insert `NodeHandles`.
                if let Some(mut handles) = world.get_non_send_mut::<NodeHandles>() {
                    handles.by_entity.remove(&e);
                }
                world.despawn(e);
            }
        }
        InboundEvent::DoorBodyEntered { id, is_player } => {
            // FR-016: the drain resolves the id; an unknown id is dropped silently.
            if let Some(entity) = world.resource::<EntityIndex>().entity(id) {
                world
                    .resource_mut::<Messages<DoorBodyEntered>>()
                    .write(DoorBodyEntered { entity, is_player });
            }
        }
        InboundEvent::BlastAnimationFinished { id } => {
            // FR-026: no decision to make — mark for removal; unknown id dropped.
            if let Some(entity) = world.resource::<EntityIndex>().entity(id) {
                world.entity_mut(entity).insert(Remove);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::{Entity, With};
    use godot::obj::InstanceId;

    use super::*;

    fn world_with_index() -> World {
        let mut world = World::new();
        world.insert_resource(EntityIndex::default());
        world.insert_resource(Messages::<DoorBodyEntered>::default());
        world
    }

    fn register(world: &mut World, id: i64) -> Entity {
        let entity = world.spawn_empty().id();
        world
            .resource_mut::<EntityIndex>()
            .register_if_absent(InstanceId::from_i64(id), || entity);
        entity
    }

    fn entities_marked_remove(world: &mut World) -> usize {
        world.query_filtered::<Entity, With<Remove>>().iter(world).count()
    }

    #[test]
    fn blast_animation_finished_marks_remove() {
        let mut world = world_with_index();
        let e = register(&mut world, 7);
        apply_non_register(&mut world, InboundEvent::BlastAnimationFinished { id: InstanceId::from_i64(7) });
        assert!(world.get::<Remove>(e).is_some());
    }

    #[test]
    fn blast_animation_finished_for_unknown_id_is_dropped() {
        let mut world = world_with_index();
        register(&mut world, 7);
        apply_non_register(&mut world, InboundEvent::BlastAnimationFinished { id: InstanceId::from_i64(99) });
        assert_eq!(entities_marked_remove(&mut world), 0);
    }

    #[test]
    fn unregister_despawns_and_drops_index_entry() {
        let mut world = world_with_index();
        let e = register(&mut world, 7);
        apply_non_register(&mut world, InboundEvent::Unregister { id: InstanceId::from_i64(7) });
        // Despawned: the entity is gone (`World::entities().len()` would also count bevy-internal
        // entities in 0.19, so liveness is what is asserted).
        assert!(world.get_entity(e).is_err());
        assert_eq!(world.resource::<EntityIndex>().entity(InstanceId::from_i64(7)), None);
    }
}
