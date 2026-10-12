# Development Guidelines

- Respectfully challenge a user request when you have a concrete reason to disagree. Explain the concern and relevant tradeoffs, suggest a better approach, and respect the user's informed decision.
- Treat the current architecture and APIs as authoritative. When adding or replacing a feature, update its callers, state, tests, and documentation to the new design together. Do not add legacy fields, tuple values, compatibility paths, duplicate implementations, or transitional shims unless the task explicitly requires them.
- Keep changes simple, readable, and cohesive. Prefer the smallest design that fits the existing code, and avoid speculative abstractions or unrelated refactors.
- Prioritize correct behavior and clear state transitions before optimizing performance or reducing memory use. Add those improvements later when profiling or measurements show they are needed, and keep them separate from behavior changes when practical.
- Read the relevant implementation, tests, and documentation before editing. Preserve unrelated worktree changes and keep each change within the layer that owns the behavior.
- Test behavior through public APIs or established application entry points. Use deterministic seeded state for randomness and cover both the normal condition and an important boundary or negative condition when the behavior is testable.
- Do not skimp on tests. Err on the side of more meaningful coverage rather than too little: cover normal behavior, edge cases, failure paths, and regressions relevant to the change. Favor tests that can catch distinct bugs over duplicate checks or tests that merely mirror the implementation.
- Run the narrowest relevant formatter and tests while iterating, then run the full applicable test suite before handing off. Treat warnings and failures as information to investigate rather than working around them.
- Respect existing module boundaries. Keep event dispatch, type definitions, and object construction in their designated modules.
- Keep feature-specific types with the feature that owns them; place shared types in the appropriate common module.
- Follow the project's established test layout and naming conventions.
- Preserve meaningful collection ordering and follow the existing state ownership and mutation model.
