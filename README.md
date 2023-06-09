# Event sourcing.rs

It is an opinionated library used to achieve cqrs/es in Rust.

A set of example snippets could be found in the `example` folder.

## Install

Event Sourcing RS uses under the hood [`sqlx`].

[`sqlx`]: https://github.com/launchbadge/sqlx

```toml
# Cargo.toml
[dependencies]
# postgres database
esrs = { version = "0.12", features = ["postgres"] }
sqlx = { version = "0.6", features = ["postgres", "runtime-tokio-native-tls", "uuid", "json", "chrono"] }
```

## Tracing

A tracing span is produced every time a projector is used or a policy is applied to a given event.

## Run examples, tests and linting

Start the docker-compose stack

```shell
docker compose run --service-ports web bash
```

Run tests.

```shell
cargo make test
```

Run linters.

```shell
cargo make clippy
```

