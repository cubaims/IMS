# cuba-mrp source layout

Current canonical implementation:

- `domain/entities.rs`
  - MRP errors
  - value objects
  - `MrpRun`
  - `MrpSuggestion`
  - MRP statuses and suggestion types

- `application/ports.rs`
  - repository traits
  - gateway traits
  - query structs
  - `RunMrpUseCase`
  - `ConfirmMrpSuggestionUseCase`
  - `CancelMrpSuggestionUseCase`

- `infrastructure/postgres.rs`
  - `PostgresMrpStore`
  - `PostgresMrpIdGenerator`
  - PostgreSQL repository and gateway implementations

- `interface/`
  - DTOs
  - HTTP handlers
  - routes

Do not reintroduce separate `domain/errors.rs`, `domain/value_objects.rs`,
`application/commands.rs`, or `application/services.rs` unless the module
layout is intentionally split again and `mod.rs` is updated accordingly.
