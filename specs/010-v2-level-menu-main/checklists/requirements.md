# Specification Quality Checklist: Milestone V2-E — level, menu, main: the scene manager and the end of v2

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-18
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details beyond what this project's constitution mandates a v2 spec
      state (Rust type/module shapes, exact source lines, gdext attribute names) — this project
      IS a Godot→Rust port governed by a constitution requiring exactly this level of citation
      (established, accepted precedent: specs 006–009 are written identically)
- [x] Focused on user value and business needs — the "user" for this constitution-governed
      porting project is the developer executing the phase; value is behavioral parity plus the
      named type-system/FFI improvements, consistent with prior milestones
- [x] Written for the project's actual stakeholders (developer + constitution reviewer), per
      established Spec Kit usage in this repository
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain — every "decide" point the milestone brief raised
      (the scene-manager's exact encoding, the typed deferred-call mechanism, `gi_plan`'s field
      shape, the `add_child` unification, the `randomize()` removal, backlog #21's actual
      remaining work once its literal premise was found already resolved) is either decided
      explicitly with reasoning, or deferred to plan-time with the BEHAVIOR pinned as the
      contract (matching V2-C/V2-D's established precedent for `HitTarget`'s shape and
      `RobotState`'s encoding)
- [x] Requirements are testable and unambiguous — each FR cites the exact `v1` source lines it
      reproduces
- [x] Success criteria are measurable (test counts, line-count targets, grep-verifiable
      crate-wide dynamic-access counts)
- [x] Success criteria are technology-specific by necessity (grep patterns, `cargo test` counts,
      line-count targets) — appropriate for this project's nature, consistent with SC-001..SC-00N
      in every prior v2 milestone spec
- [x] All acceptance scenarios are defined (4 for US1, 8 for US2, 5 for US3)
- [x] Edge cases are identified (4, covering the GI-plan's stateful third input, the two
      `randomize()` removals' seeded-vs-unseeded distinction, the respawn-hitch observation, and
      confirming US1 doesn't touch `menu.tscn`'s own unrelated connection path)
- [x] Scope is clearly bounded (Assumptions' "Out of scope" list — every other v2 module named
      explicitly as untouched)
- [x] Dependencies and assumptions identified (five plan-time deferrals, each with the specific
      behavioral contract the deferred decision must satisfy)

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria (FR-001..FR-020, each
      traceable to a numbered Acceptance Scenario)
- [x] User scenarios cover primary flows (boot→menu→play→level→quit→menu; robot/player spawn,
      GI setup, robot respawn, forklift model pick; every settings row's read/write round-trip)
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak beyond this project's established constitution-mandated
      citation level

## Notes

- This checklist's "no implementation details" / "technology-agnostic" items are interpreted in
  the context of this specific project: a Rust port of a GDScript game governed by a
  constitution that MANDATES citing exact source lines, Rust type shapes and
  `cargo`/grep-verifiable success criteria in every v2 spec (Principle III). Prior milestones
  (006 through 009) were written and accepted identically. All items pass under that
  established convention.
- This is the LAST v2 milestone spec — after it, `docs/v2-backlog.md` and `README.md` both
  need updating to reflect v2's completion (FR-020, SC-008), a cross-cutting requirement this
  spec's own final commit owns rather than deferring to a later milestone.
