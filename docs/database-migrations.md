# Database Migrations

This is the operational contract for IMS database migrations and SQLx state.

## Policy

- `migrations/0001_schema_final_ultimate_complete_v9.sql` is the frozen
  baseline. It rebuilds the IMS schemas and must not be edited after a database
  has applied it.
- Every later schema or seed-data change must be appended as a new numbered
  migration. Do not patch an already-applied migration file to change live
  behavior.
- Data patch migrations should be idempotent and should state their acceptance
  check in a script or runbook.
- Normal API startup is migration-free. Run migrations explicitly with
  `cargo run -p cuba-api -- migrate`, or intentionally set `RUN_MIGRATIONS=true`
  for a one-off startup migration.

## State Checks

Use the checksum script before running migrations against a database that may
already contain IMS schemas:

```bash
./scripts/check_sqlx_migration_checksums.sh
```

The script compares local migration files with `public._sqlx_migrations` using
SQLx 0.8's checksum algorithm, SHA-384 over the SQL file bytes.

Important statuses:

- `OK`: the local file matches the applied SQLx checksum and description.
- `PENDING`: the file exists locally but the database has not recorded it.
- `CHECKSUM_MISMATCH`: the database recorded this version with different file
  bytes. Do not edit the migration file to make the error go away.
- `APPLIED_ONLY`: the database has a version that is no longer in `migrations/`.
  SQLx will reject this unless the missing file is restored.
- `MISSING_SQLX_TABLE`: IMS schemas exist but SQLx bookkeeping is absent. Do not
  run `cargo run -p cuba-api -- migrate` until the baseline is repaired.

The patch migration acceptance script checks the current data effects for audit
permissions, system-parameter permissions, quality/demo-login seed data, and
the SQLx rows for versions `0003`, `0007`, and `0008` when the SQLx table is
present:

```bash
./scripts/verify_patch_migrations.sh
```

To also verify the demo password through the running API:

```bash
VERIFY_PATCH_API=1 ./scripts/verify_patch_migrations.sh
```

The API check defaults to `IMS_BASE_URL` or `http://localhost:8080`, and the
demo password defaults to `IMS_DEMO_PASSWORD` or `password`.

## One-Time Repair Plan

Use this only to repair existing developer/demo databases that were initialized
before the migration policy was fixed.

1. Back up the database and stop writers.
2. Run `./scripts/check_sqlx_migration_checksums.sh` and save the output.
3. If migrations are `PENDING`, apply them normally with
   `cargo run -p cuba-api -- migrate`. Do not edit older migration files.
4. If `CHECKSUM_MISMATCH` is reported, first prove the database effects already
   match the intended migration state. Then generate guarded bookkeeping SQL:

   ```bash
   ./scripts/check_sqlx_migration_checksums.sh --repair-sql
   ```

   Review the SQL and run it once. It updates `public._sqlx_migrations.checksum`
   only for rows whose old checksum still matches the inspected state.
5. If `MISSING_SQLX_TABLE` is reported while IMS schemas exist, do not run SQLx
   migrations. Manually baseline only the migrations whose effects are already
   present, using SHA-384 checksums from the local files and `success = true`.
   Then run the checksum script again. Insert no row for a migration whose data
   or schema effect is absent.
6. Run `./scripts/verify_patch_migrations.sh`. For a full auth/runtime check,
   start `cuba-api` and run it with `VERIFY_PATCH_API=1`.
7. Keep the repair SQL and final script output with the environment's deployment
   notes. Future changes must be new migration files.
