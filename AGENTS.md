# Balatro Bot Development Notes

- Treat the current architecture and APIs as authoritative. When adding or replacing a feature, update its callers, state, tests, and documentation to the new design together. Do not add legacy fields, tuple values, compatibility paths, duplicate implementations, or transitional shims unless the task explicitly requires them.
- Keep changes simple, readable, and cohesive. Prefer the smallest design that fits the existing code, and avoid speculative abstractions or unrelated refactors.
- Prioritize correct behavior and clear state transitions before optimizing performance or reducing memory use. Add those improvements later when profiling or measurements show they are needed, and keep them separate from behavior changes when practical.
- Read the relevant implementation, tests, and documentation before editing. Preserve unrelated worktree changes and keep each change within the layer that owns the behavior.
- Use public event or domain paths in behavioral tests. Use deterministic seeded state for randomness and cover both the normal condition and an important boundary or negative condition when the behavior is testable.
- Run the narrowest relevant formatter and tests while iterating, then run the full applicable test suite before handing off. Treat warnings and failures as information to investigate rather than working around them.
- Keep trigger and update dispatch in `balatro_engine/src/core/joker_events.rs`; keep Joker definitions and construction in `joker.rs`.
- Keep Joker-only enums in `balatro_engine/src/core/joker_types.rs`; keep shared card and game enums in `enums.rs`.
- Put Rust tests in `balatro_engine/src/core/tests/` instead of appending them to production modules.
- Joker collections must preserve left-to-right order and mutate their existing state in place.
