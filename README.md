# Event sourcing.rs

It is an opinionated library used to achieve cqrs/es in Rust.

A set of example snippets can be found in the `example` folder.

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

## Tests and linting

Start a Postgres instance.

```shell
docker run -p 5432:5432 --name postgres -e POSTGRES_PASSWORD=postgres -d postgres:11-alpine
```

Export DATABASE_URL environment variable with freshly new created database.

```shell
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres
```

Run tests.

```shell
cargo test --all-targets --all-features
```

Run linters.

```
cargo clippy --all-targets --all-features -- -W clippy::nursery
```

Finally eventually unset `DATABASE_URL` environment variable.

```shell
unset DATABASE_URL
```
