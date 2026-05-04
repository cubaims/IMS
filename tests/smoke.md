# Smoke tests

1. Start PostgreSQL.
2. Run `./scripts/init_db.sh`.
3. Run `cargo run -p cuba-api`.
4. Check `GET /health`.
5. Check one module route, for example `GET /api/inventory/health`.
