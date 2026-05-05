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

### Core Business Modules

- **cuba-auth** - Authentication & Authorization (JWT, RBAC)
- **cuba-master-data** - Master Data Management (Materials, BOMs, Customers, Suppliers, etc.)
- **cuba-inventory** - Inventory Management (Stock, Movements, Bins, Batches)
- **cuba-purchase** - Purchase Order Management (PO, Receipts, GR/IR)
- **cuba-sales** - Sales Order Management (SO, Shipments, Invoicing)
- **cuba-production** - Production Order Management (MO, BOM Explosion, One-click Completion)
- **cuba-quality** - Quality Management (Inspections, QC Results)
- **cuba-mrp** - Material Requirements Planning (MRP Run, Shortage Analysis)
- **cuba-reporting** - Reporting & Analytics (Materialized Views, Data Refresh)

### Infrastructure Modules

- **cuba-api** - Main API Gateway (Axum HTTP Server)
- **cuba-worker** - Background Job Worker (Async Tasks)
- **cuba-shared** - Shared Types & Utilities (AppState, Errors, DB Helpers)

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

---

## Module Deep Dive: cuba-reporting

### Module Structure

```text
crates/cuba-reporting/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── domain/
    │   ├── mod.rs
    │   ├── entities.rs
    │   ├── errors.rs
    │   └── value_objects.rs
    ├── application/
    │   ├── mod.rs
    │   ├── commands.rs
    │   ├── ports.rs
    │   └── services.rs
    ├── infrastructure/
    │   ├── mod.rs
    │   └── postgres.rs
    └── interface/
        ├── mod.rs
        ├── dto.rs
        ├── handlers.rs
        └── routes.rs
```

### API Endpoints

| Method | Endpoint | Description | Status |
|--------|----------|-------------|--------|
| GET | `/api/reporting/health` | Module health check | ✅ Active |
| GET | `/api/reporting/current-stock` | Current stock levels | 🚧 Placeholder |
| GET | `/api/reporting/inventory-value` | Inventory valuation | 🚧 Placeholder |
| GET | `/api/reporting/quality-status` | Quality inspection status | 🚧 Placeholder |
| GET | `/api/reporting/mrp-shortage` | MRP shortage analysis | 🚧 Placeholder |
| GET | `/api/reporting/low-stock-alert` | Low stock alerts | 🚧 Placeholder |
| GET | `/api/reporting/data-consistency` | Data consistency checks | 🚧 Placeholder |
| POST | `/api/reporting/refresh` | Refresh materialized views | ✅ Active |
