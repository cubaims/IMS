# IMS Workspace Architecture

This codebase follows module-first DDD + Clean Architecture.

Each bounded context is a crate named `cuba-{module}` and contains four internal layers:

1. `domain`: business vocabulary, value objects, entities, domain errors.
2. `application`: use cases, commands, ports, orchestration.
3. `infrastructure`: PostgreSQL adapters and external integrations.
4. `interface`: DTOs, HTTP handlers, routing.

The API crate is only a delivery/composition crate. It wires bounded contexts and exposes HTTP routes.
