# API gaps

Engine API gaps found while porting: a symbol the local Godot binary has but the gdext prebuilt
API lacks. Catalogued per constitution 1.4.0, Principle I — each gap is isolated behind one typed
value, marked at the exact spot with a `// api-gap(godot-<version>): <symbol> — <reason>; replace
when the binding ships it` comment. Changing the crate's API feature set (`api-custom`, `api-4-x`)
to remove a gap is a user decision, never an implementer's default.

| Symbol | Introducing version | Workaround | Location |
|---|---|---|---|
| `Scaling3DMode::NEAREST` | Godot 4.7 (absent from gdext 0.5.5's prebuilt API 4.6) | `ScaleFilter::Nearest` maps to `Scaling3DMode::from_ord(5)`, reusing the `MAX` ordinal slot the 4.6 binding leaves unnamed (`Scaling3DMode::MAX = 5` in the generated bindings; `try_from_ord` accepts it as a bare ordinal, so the call does not panic — it just has no name for it) | `oxide_godot_core/oxide_godot_lib/src/settings/graphics.rs`, `ScaleFilter::to_engine()` |
