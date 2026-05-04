check:
    cargo check --workspace

fmt:
    cargo fmt --all

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

run:
    cargo run -p cuba-api

init-db:
    ./scripts/init_db.sh
