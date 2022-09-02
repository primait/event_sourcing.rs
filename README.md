# Event sourcing.rs

It is an opinionated library used to achieve cqrs/es in Rust.

A complete example can be found in the `example` folder.

## Install

Event Sourcing RS uses under the hood [`sqlx`].

[`sqlx`]: https://github.com/launchbadge/sqlx

```toml
# Cargo.toml
[dependencies]
# postgres database
esrs = { version = "0.6", features = ["postgres"] }
sqlx = { version = "0.6", features = ["postgres", "runtime-tokio-native-tls", "uuid", "json", "chrono"] }
```

## Run examples, tests and linting

Payment examples simulate paying with credit card goods or services updating bank account (so its balance).

To run examples and tests first you need to start new postgres instance. You'll not be able to run postgres example and
tests otherwise.

```shell
docker run -p 5432:5432 --name postgres -e POSTGRES_PASSWORD=postgres -d postgres:11-alpine
```

Export DATABASE_URL environment variable with freshly new created database.

```shell
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres
```

Run examples.

```shell
# Startup database and export `DATABASE_URL`
docker run -p 5432:5432 --name postgres -e POSTGRES_PASSWORD=postgres -d postgres:11-alpine
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres

# Run examples
cargo run -p simple_projection
cargo run -p simple_side_effect
cargo run -p simple_saga
cargo run -p aggregate_merging
```

Run tests and linting.

```shell
# Run tests
cargo test --all-targets --all-features
# Run linting
cargo clippy --all-targets --all-features -- -W clippy::nursery
```

Finally eventually unset `DATABASE_URL` environment variable.

```shell
unset DATABASE_URL
```
