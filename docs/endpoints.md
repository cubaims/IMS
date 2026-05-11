- `/api/auth`: Auth
  - Frontend integration guide: `docs/frontend-auth.md`
  - `/api/auth/roles` and `/api/auth/permissions` return the current
    authenticated user's effective roles and permissions. They are not system
    role-management endpoints.
  - System role/user administration lives under `/api/system/roles` and
    `/api/system/users`; those routes require `ADMIN`.
  - Access token invalidation model: lightweight short-lived self-contained JWT.
    The auth middleware validates signature, issuer, expiry, and token type, then
    trusts the roles and permissions embedded in the access token for its short
    lifetime. It does not query the database on every request.
  - Default access token lifetime is 900 seconds (`JWT_EXPIRES_SECONDS`, 15
    minutes). Refresh tokens default to 30 days (`JWT_REFRESH_EXPIRES_SECONDS`).
  - User disablement and permission changes are enforced at refresh/login time:
    `/api/auth/refresh` reloads the user, roles, and permissions from PostgreSQL,
    rejects disabled users, and rotates the refresh token. Already-issued access
    tokens may remain valid until expiry.
  - Refresh token rotation revokes the previous refresh token in
    `sys.sys_refresh_tokens.revoked_at` and links the replacement through
    `replaced_by`; replaying the old refresh token returns
    `REFRESH_TOKEN_INVALID`.
  - Strong immediate invalidation would require the auth middleware to query the
    user row on every request and optionally compare a token version; that is not
    the current runtime model.
- `/api/master-data`: MasterData
  - OpenAPI: `/api/openapi/master-data.json`
  - Postman collection: `docs/postman/master-data.phase3.postman_collection.json`
  - Bruno collection: `docs/bruno/master-data.phase3/`
  - Acceptance script: `TOKEN=<write-or-admin-jwt> READ_TOKEN=<read-only-jwt> ./scripts/phase3_acceptance.sh`
- `/api/inventory`: Inventory
  - Inventory count OpenAPI: `docs/openapi/inventory-count.phase7.openapi.json`
  - Postman collection: `docs/postman/inventory-count.phase7.postman_collection.json`
  - Bruno collection: `docs/bruno/inventory-count.phase7/`
  - Acceptance/regression script: `./scripts/verify_phase7_inventory_count.sh`
  - Required count permissions:
    `inventory-count:read`, `inventory-count:write`, `inventory-count:submit`,
    `inventory-count:approve`, `inventory-count:post`, `inventory-count:close`.
  - Phase 7 count flow:
    `POST /api/inventory/counts` -> `POST /api/inventory/counts/{id}/generate-lines`
    -> `PATCH /api/inventory/counts/{id}/lines/{line_no}` -> `POST /submit`
    -> `POST /approve` -> `POST /post` -> `POST /close`.
  - Phase 7 regression:
    1. Apply the inventory count contract to existing developer databases when
       they were created before Phase 7:
       `psql "$DATABASE_URL" -f migrations/0002_phase7_inventory_count_contract.sql`.
    2. Start `cuba-api`: `cargo run -p cuba-api`. Normal API startup skips
       migrations by default. If an older local shell still exports
       `RUN_MIGRATIONS=true`, start with
       `RUN_MIGRATIONS=false cargo run -p cuba-api` to avoid changing an
       already-initialized database.
    3. Run `./scripts/verify_phase7_inventory_count.sh`.
    4. Run real PostgreSQL repository regression:
       `PHASE7_RUN_DB_TESTS=1 cargo test -p cuba-inventory --test phase7_inventory_count_postgres -- --test-threads=1`.
  - Phase 7 database contract:
    `wms.post_inventory_transaction(transaction_id, movement_type, material_id,
    quantity, from_bin, to_bin, batch_number, serial_number, operator,
    quality_status, reference_doc, notes, transaction_date, unit_price)`.
    Movement `701` is count gain with `to_bin`; movement `702` is count loss
    with `from_bin`.
  - Quantity contract: Phase 7 only accepts integer inventory quantities.
    Rust rejects fractional counted quantities and PostgreSQL count columns are
    integer. If decimal inventory is required later, update schema, domain
    rules, DTOs, and `wms.post_inventory_transaction()` together.
  - Scope conflict contract: Phase 7 blocks duplicate open counts for the same
    exact scope dimensions. Overlap checks such as `ZONE` covering an open
    `BIN`, or `MATERIAL` covering an already counted stock row, remain future
    hardening work.
  - Expected structured errors:
    `INVENTORY_COUNT_NOT_FOUND` -> 404,
    `COUNTED_QTY_INVALID` -> 400,
    `INVENTORY_COUNT_STATUS_INVALID` -> 409,
    `INVENTORY_COUNT_DUPLICATED_SCOPE` -> 409,
    `COUNT_DIFFERENCE_POST_FAILED` -> 500.
- `/api/purchase`: Purchase
- `/api/sales`: Sales
- `/api/production`: Production
- `/api/quality`: Quality
- `/api/traceability`: Traceability query orchestration
  - `GET /api/traceability/health`
  - `GET /api/traceability/batches/{batch_number}`
  - `GET /api/traceability/serials/{serial_number}`
  - Required permission: `traceability:read`.
- `/api/mrp`: Mrp
- `/api/reports`: Reporting
