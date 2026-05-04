# IMS Workspace Server

Rust 1.95 / Edition 2024 service skeleton for IMS (Workspace), using DDD + Clean Architecture.

## Architecture

This project uses a module-first workspace layout. Each bounded-context crate owns its own clean architecture layers:

```text
crates/cuba-{module}/
├── src/domain/          # Entities, value objects, domain errors/rules
├── src/application/     # Use cases, ports, commands, service orchestration
├── src/infrastructure/  # PostgreSQL adapters and external dependencies
├── src/interface/       # DTOs, HTTP routes, handlers
└── src/lib.rs
```

The database schema is under `migrations/0001_schema_final_ultimate_complete_v9.sql`.

## Modules

- cuba-auth
- cuba-master-data
- cuba-inventory
- cuba-purchase
- cuba-sales
- cuba-production
- cuba-quality
- cuba-mrp
- cuba-reporting
- cuba-api
- cuba-worker

## Run locally

```bash
cp .env.example .env
docker compose up -d db
./scripts/init_db.sh
cargo run -p cuba-api
```

## Verify

```bash
cargo check --workspace
cargo test --workspace
```
