#!/usr/bin/env bash

docker run -p 5432:5432 --name postgres -e POSTGRES_PASSWORD=postgres -d postgres:11-alpine && \
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres && \
cargo run --example postgres-payments && \
cargo run --example sqlite-payments && \
cargo test --workspace --all-targets --all-features && \
cargo clippy --workspace --all-targets --all-features -- -W clippy::nursery && \
unset DATABASE_URL && \
docker stop postgres && \
docker rm postgres
