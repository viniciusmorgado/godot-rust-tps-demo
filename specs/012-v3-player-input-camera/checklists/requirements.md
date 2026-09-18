# Specification Quality Checklist: Milestone V3-B — the player: `player`, `player_input`, `camera_noise_shake` as one entity over three nodes

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-18
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details beyond what this project's constitution mandates a v3 spec
      state (exact v2 source lines, Rust type/attribute names, scene lines, engine calls per
      phase) — this project IS a Godot→Rust port governed by a constitution whose "ECS shape
      (v3)" subsection requires exactly this level of citation (established, accepted
      precedent: specs 006–011)
- [x] Focused on user value and business needs — the "user" is the developer executing the
      phase; value is behavioral parity plus the ECS tick shape every later gameplay module
      copies (two `EngineQuery` sets, sub-bridges, replication as projection)
- [x] Written for the project's actual stakeholders (developer + constitution reviewer)
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain — every "spec decides" point of the brief (set
      labeling of the two `EngineQuery` sets; camera rotation applied in `EngineQuery` before
      the raycast; `CameraNoiseShake::add_trauma` removed; sub-bridge root-id resolution once)
      is decided with its v2 line or deferred to plan-time with the BEHAVIOR pinned
- [x] Requirements are testable and unambiguous — each FR cites the v2 lines it reproduces
      and the harness case or test that proves it
- [x] Success criteria are measurable (test counts, grep-verifiable callback absence,
      six harness diffs, the US4 experiment log, two checkpoints)
- [x] Success criteria are technology-specific by necessity, consistent with prior specs
- [x] All acceptance scenarios are defined (6 for US1, 5 for US2, 4 for US3, 3 for US4)
- [x] Edge cases are identified (9: `set_player_id` before `ready`, same-step land+jump,
      `call_local` re-entrancy, cooldown read/write order, bullet spawn frame, respawn order,
      remote input on the server, mouse look scale, backlog #6)
- [x] Scope is clearly bounded (Assumptions' "Out of scope"; FR-020's untouched-file list)
- [x] Dependencies and assumptions identified (eight research-time verifications, the US4
      experiment as the central one)

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria (FR-001..FR-026, each
      traceable to a scenario, the Context table, or the top block)
- [x] User scenarios cover primary flows (tick, replay, input/camera, shake, animation order)
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak beyond the established citation level

## Notes

- The `AnimationTree` ordering (US4/FR-018) is deliberately left as a research question with
  an expected answer (B) and a parity criterion, not a decision: the spec cannot honestly pin
  what only the two-tree experiment can show.
- The "Measured timing differences" table is intentionally empty at spec time.
