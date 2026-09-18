//! The inbound event queue (research.md R2, FR-006).
//!
//! A module-private `thread_local!` `RefCell<Vec<_>>`: `push` borrows it for one `Vec::push` and
//! returns, so an engine callback — even one fired synchronously by an engine call the sync layer
//! itself made while a schedule is running — can never double-borrow `EcsWorld` or the `World`.
//! Nothing in this file names either. Every engine callback runs on the main thread (the
//! workspace's `experimental-threads` feature only widens gdext's API surface), so a `Mutex` would
//! buy nothing and would demand `InboundEvent: Send`, which `Gd<T>` is not.

use std::cell::RefCell;

use super::event::InboundEvent;

thread_local! {
    static QUEUE: RefCell<Vec<InboundEvent>> = const { RefCell::new(Vec::new()) };
}

/// Bridges call this from `ready`, `exit_tree` and their `#[func]`/signal handlers.
pub fn push(event: InboundEvent) {
    QUEUE.with(|q| q.borrow_mut().push(event));
}

/// Driver only: takes every queued event in FIFO order (a swap, never a clone) and leaves the
/// queue empty.
pub fn drain() -> Vec<InboundEvent> {
    QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use godot::obj::InstanceId;

    use super::*;

    /// (variant tag, id) — `InboundEvent` has no `PartialEq` (it can hold `Gd<T>`).
    fn key(ev: &InboundEvent) -> (u8, i64) {
        match ev {
            InboundEvent::Register { id, .. } => (0, id.to_i64()),
            InboundEvent::Unregister { id } => (1, id.to_i64()),
            InboundEvent::DoorBodyEntered { id, .. } => (2, id.to_i64()),
            InboundEvent::BlastAnimationFinished { id } => (3, id.to_i64()),
        }
    }

    // `thread_local!` state is per test thread: each test pushes and drains within itself.

    #[test]
    fn drain_returns_events_in_push_order() {
        push(InboundEvent::Unregister { id: InstanceId::from_i64(1) });
        push(InboundEvent::DoorBodyEntered { id: InstanceId::from_i64(2), is_player: true });
        push(InboundEvent::BlastAnimationFinished { id: InstanceId::from_i64(3) });
        let drained: Vec<(u8, i64)> = drain().iter().map(key).collect();
        assert_eq!(drained, vec![(1, 1), (2, 2), (3, 3)]);
    }

    #[test]
    fn drain_empties_the_queue() {
        push(InboundEvent::Unregister { id: InstanceId::from_i64(9) });
        assert_eq!(drain().len(), 1);
        assert!(drain().is_empty());
    }
}
