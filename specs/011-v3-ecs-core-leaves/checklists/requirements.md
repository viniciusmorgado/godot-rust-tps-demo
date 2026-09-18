# Specification Quality Checklist: Milestone V3-A — ECS core and the three leaf effects

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-18
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details beyond what this project's constitution mandates a v3 spec
      state (Rust type/module shapes, exact source lines, crate/feature pins, gdext and
      `bevy_ecs` symbol citations) — this project IS a Godot→Rust port governed by a
      constitution whose "ECS shape (v3)" subsection requires exactly this level of citation
      (established, accepted precedent: specs 006–010 are written identically)
- [x] Focused on user value and business needs — the "user" for this constitution-governed
      porting project is the developer executing the phase; value is behavioral parity plus the
      ECS infrastructure every later v3 milestone reuses, consistent with prior milestones
- [x] Written for the project's actual stakeholders (developer + constitution reviewer), per
      established Spec Kit usage in this repository
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain — every "spec decides" point the milestone brief
      raised (which schedule drains the queue and why; double-registration idempotent vs
      rejected; camera handle cached vs re-fetched) is decided explicitly with reasoning in
      the Assumptions and the corresponding acceptance scenario; the exact Rust mechanisms
      are deferred to plan-time with the BEHAVIOR pinned as the contract (V2 precedent)
- [x] Requirements are testable and unambiguous — each FR cites the exact `v2` source lines it
      reproduces or the scenario it traces to
- [x] Success criteria are measurable (test counts, grep-verifiable absence of per-frame
      callbacks and `run_schedule` call sites, diff-empty file sets, harness diff outcomes)
- [x] Success criteria are technology-specific by necessity (grep patterns, `cargo test`
      counts, `cargo tree` crate counts) — appropriate for this project's nature, consistent
      with every prior milestone spec
- [x] All acceptance scenarios are defined (10 for US1, 5 for US2, 5 for US3, 5 for US4)
- [x] Edge cases are identified (6: signal during a schedule run, unregistration after
      despawn in both orders, no camera, timer boundary rounding, double entry in one step,
      registration of an already-dead node)
- [x] Scope is clearly bounded (Assumptions' "Out of scope" list names every other v3 module,
      authority components, `EngineQuery` members, backlog #30)
- [x] Dependencies and assumptions identified (four research-time verifications from the
      brief, each with the expected answer and the check; two corrections to the brief
      recorded — `bullet.tscn` does not bind `Blast`, and the constitution's
      `experimental-threads` wording — with their consequences)

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria (FR-001..FR-030, each
      traceable to a numbered acceptance scenario, the Context table, or the top block)
- [x] User scenarios cover primary flows (autoload + driver + queue + maps + timer; door
      signal→event→system→SyncOut; puff timer phases; blast SyncIn/SyncOut + signal despawn)
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak beyond this project's established constitution-mandated
      citation level

## Notes

- This checklist's "no implementation details" / "technology-agnostic" items are interpreted in
  the context of this specific project: a Rust port of a GDScript game governed by a
  constitution that MANDATES citing exact source lines, Rust type shapes and
  `cargo`/grep-verifiable success criteria in every spec (Principle III, "ECS shape (v3)").
  Prior milestones (006 through 010) were written and accepted identically. All items pass
  under that established convention.
- The "Measured timing differences" table is intentionally empty at spec time: FR-030 requires
  it to be filled (or confirmed empty by three empty diffs) before the closing commit.
- The spec records that the brief's claim "`Blast` is referenced by `bullet.tscn`" is false
  (`bullet.tscn:514` is a plain `Node3D`); visual checkpoint (2) was adjusted accordingly.
