//! `InstanceId → Entity`, the plain-resource half of the identity maps (research.md R4, FR-007).

use std::collections::HashMap;

use bevy_ecs::prelude::{Entity, Resource};
use godot::obj::InstanceId;

#[derive(Resource, Default)]
pub struct EntityIndex {
    by_id: HashMap<InstanceId, Entity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Registration {
    Registered(Entity),
    AlreadyRegistered(Entity),
}

impl EntityIndex {
    /// Idempotent registration (FR-008): an already-registered id keeps its first entity and
    /// `spawn` is not called.
    pub fn register_if_absent(
        &mut self,
        id: InstanceId,
        spawn: impl FnOnce() -> Entity,
    ) -> Registration {
        if let Some(&existing) = self.by_id.get(&id) {
            return Registration::AlreadyRegistered(existing);
        }
        let entity = spawn();
        self.by_id.insert(id, entity);
        Registration::Registered(entity)
    }

    /// `None` = unknown id, a no-op (FR-008).
    pub fn unregister(&mut self, id: InstanceId) -> Option<Entity> {
        self.by_id.remove(&id)
    }

    pub fn entity(&self, id: InstanceId) -> Option<Entity> {
        self.by_id.get(&id).copied()
    }

    /// Used by the validity sweep and by `Remove`: drops every key mapping to `entity`.
    pub fn remove_entity(&mut self, entity: Entity) {
        self.by_id.retain(|_, e| *e != entity);
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::World;

    use super::*;

    fn id(n: i64) -> InstanceId {
        InstanceId::from_i64(n)
    }

    #[test]
    fn register_spawns_once() {
        let mut world = World::new();
        let mut index = EntityIndex::default();
        let mut spawns = 0;
        let reg = index.register_if_absent(id(1), || {
            spawns += 1;
            world.spawn_empty().id()
        });
        let Registration::Registered(e) = reg else {
            panic!("first registration must spawn");
        };
        assert_eq!(spawns, 1);
        // `World::entities().len()` also counts bevy-internal entities in 0.19, so liveness of
        // the ONE spawned entity is asserted instead of a raw count.
        assert!(world.get_entity(e).is_ok());
    }

    #[test]
    fn second_register_of_same_id_keeps_first_entity() {
        let mut world = World::new();
        let mut index = EntityIndex::default();
        let Registration::Registered(e1) = index.register_if_absent(id(1), || world.spawn_empty().id())
        else {
            panic!("first registration must spawn");
        };
        let mut second_spawn_called = false;
        let reg = index.register_if_absent(id(1), || {
            second_spawn_called = true;
            world.spawn_empty().id()
        });
        assert_eq!(reg, Registration::AlreadyRegistered(e1));
        assert!(!second_spawn_called);
        assert!(world.get_entity(e1).is_ok());
        assert_eq!(index.entity(id(1)), Some(e1));
    }

    #[test]
    fn unregister_unknown_id_is_noop() {
        let mut world = World::new();
        let mut index = EntityIndex::default();
        index.register_if_absent(id(1), || world.spawn_empty().id());
        assert_eq!(index.unregister(id(42)), None);
        assert!(index.entity(id(1)).is_some());
    }

    #[test]
    fn unregister_then_late_sweep_is_noop() {
        // `exit_tree` unregistration drained first, then the validity sweep finds the entity.
        let mut world = World::new();
        let mut index = EntityIndex::default();
        index.register_if_absent(id(1), || world.spawn_empty().id());
        let e = index.unregister(id(1)).expect("registered");
        index.remove_entity(e);
        assert_eq!(index.entity(id(1)), None);
        assert!(index.by_id.is_empty());
    }

    #[test]
    fn sweep_then_late_unregister_is_noop() {
        // The validity sweep despawned first (node freed elsewhere), then `exit_tree` arrives.
        let mut world = World::new();
        let mut index = EntityIndex::default();
        let Registration::Registered(e) = index.register_if_absent(id(1), || world.spawn_empty().id())
        else {
            panic!("first registration must spawn");
        };
        index.remove_entity(e);
        assert_eq!(index.unregister(id(1)), None);
        assert!(index.by_id.is_empty());
    }
}
