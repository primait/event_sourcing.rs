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
esrs = { version = "0.3", features = [ "postgres" ] }
# sqlite database
esrs = { version = "0.3", features = [ "sqlite" ] }
```

## Run examples and tests

Payment examples simulate paying with credit card goods or services updating bank account (so its balance).

To run examples and tests first you need to start new postgres instance. 
You'll not be able to run postgres example and tests otherwise.

```shell
docker run --name postgres -e POSTGRES_PASSWORD=postgres -d postgres:11-alpine
```

Export DATABASE_URL environment variable with freshly new created database.

```shell
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres
```

Run both examples, all tests and linting.

```shell
cargo run --example postgres-payments && \
cargo run --example sqlite-payments && \
cargo test --all-targets --all-features && \
cargo clippy --all-targets --all-features
```

Then eventually unset database_url variable.

```shell
unset DATABASE_URL
```

