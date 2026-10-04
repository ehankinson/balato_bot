# Balatro Bot Development Notes

- Use the current event-driven Joker API only. Do not add or restore legacy Joker update fields, tuple values, or compatibility paths.
- Keep trigger and update dispatch in `balatro_engine/src/core/joker_events.rs`; keep Joker definitions and construction in `joker.rs`.
- Keep Joker-only enums in `balatro_engine/src/core/joker_types.rs`; keep shared card/game enums in `enums.rs`.
- Put Rust tests in `balatro_engine/src/core/tests/` instead of appending them to production modules.
- Joker collections must preserve left-to-right order and mutate their existing state in place.
