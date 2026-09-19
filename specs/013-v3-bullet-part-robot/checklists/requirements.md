# Specification Quality Checklist: Milestone V3-C — the enemy: `bullet`, `part`, `red_robot` over the ECS core

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-19
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details beyond what this project's constitution mandates a v3 spec
      state (exact v2 source lines, Rust type/attribute names, scene lines, engine calls per
      phase) — this project IS a Godot→Rust port governed by a constitution whose "ECS shape
      (v3)" subsection requires exactly this level of citation (established precedent: specs
      006–012)
- [x] Focused on user value and business needs — the "user" is the developer executing the
      phase; value is behavioral parity plus the completion of the ECS list (after V3-C every
      gameplay module is ECS) with the patterns every later review copies (entity-to-entity
      events, engine RNG in glue, a second `AnimationTree`)
- [x] Written for the project's actual stakeholders (developer + constitution reviewer)
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain — every "spec decides" point of the brief is
      decided with its v2 line (the parts as self-registering entities, not sub-bridges; the
      `PartExplode` event carrying the robot-side RNG draws; the bullet's collider as an
      `InstanceId` resolved in `SyncOut`; the `_clip_ray` write in `SyncOut`; the root-motion
      read in the first `EngineQueryOrient`; `hit` kept `call_local`, `explode`/`destroy`/
      `play_shoot` to `call_remote`; backlog #31 default deferred with FR-023's variant) or
      deferred to plan-time with the BEHAVIOR pinned
- [x] Requirements are testable and unambiguous — each FR cites the v2 lines it reproduces
      and the harness case or test that proves it
- [x] Success criteria are measurable (test counts, grep-verifiable callback absence, five
      harness diffs, the US4 experiment log, two checkpoints)
- [x] Success criteria are technology-specific by necessity, consistent with prior specs
- [x] All acceptance scenarios are defined (6 for US1, 6 for US2, 9 for US3, 3 for US4)
- [x] Edge cases are identified (12: same-step expiry+collision, hitting a dead robot,
      synchronous `rpc_hit`, the robot's `hit` timing, `exploded` from glue, the parts' RNG
      order, the client part's timer, `resume_approach` two paths, `shoot_check`, the shoot
      `AnimationPlayer`, trauma pending across death, dead-at-ready robots)
- [x] Scope is clearly bounded (Assumptions' "Out of scope"; FR-025's untouched-file list;
      constitution 1.5.2's exclusion of `flying_forklift`)
- [x] Dependencies and assumptions identified (eight research-time verifications, the US4
      experiment and the `RigidBody3D` determinism question as the central ones)

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria (FR-001…FR-031 map to the
      scenarios, the harness cases (a)–(e), the checkpoints and the review greps)
- [x] User scenarios cover primary flows (bullet lifecycle; part explode/fade/destroy on both
      peers; robot approach/aim/shoot/hit/death and the replay path; the tree ordering)
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak beyond the constitution-mandated level

## Notes

- All items pass on the first validation iteration (2026-09-19). The only open decision for
  the user is backlog #31 (FR-023), defaulted to deferred; it is a plan-review decision, not a
  clarification marker.
