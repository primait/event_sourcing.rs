# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `AggregateManager` should implement `name` function that act as `Identifier`.
- `AggregateManager` should implement `EventStore` associated type.
- `Projector` should implement `delete` function.
- `PgStore::setup` function to create table and indexes if not exists.
- `PgStore::add_projector` function to add a projector to store projectors list.
- `PgStore::add_policy` function to add a policy to store policies list.
- `PgStore::set_projectors` function to set the store projectors list.
- `PgStore::set_policies` function to set the store policies list.

### Changed

- `Aggregate` is now pure. API changed so user have to implement `Aggregate` for logic and `AggregateManager` in 
  order to handle persistence layer.
- `PgStore::new` takes ownership of pool; removed projectors and policies from params.
- `Projector` second parameter changed from `Transaction` to `PgConnection`.
- `PgStore` moved to `esrs::store::postgres` module.
- `PgStore::new` function is now sync and its return value is no longer a `Result` but `Self`. Removed type param.
- `PgStore<Event, Error>` became `PgStore<Aggregate>`.
- `Projector` moved to `esrs::store::postgres` module.
- `Policy` moved to `esrs::store::postgres` module.
- `Aggregate::apply_event` `payload` parameter changed from reference to value (`Self::Event`).
- `AggregateManager::event_store` changed to return a reference to it's associated type `EventStore`.

### Removed

- `Aggregate::validate_command` is removed; now validation should be made in `handle_command`.
- `Sqlite` feature and its implementation.
- `Identifier` trait.
- `ProjectorEraser` trait.
- `EraserStore` trait.
- `ProjectorStore` trait.
- `Eraser` trait.
- `AggregateState::new_with_state` removed due to potential inconsistency while loading state.

## [0.6.2]

### Changed

- Bump min version of supported Rust to 1.58 since <1.58 fails to resolve sqlx-core dep

[Unreleased]: https://github.com/primait/event_sourcing.rs/compare/0.6.2...HEAD
[0.6.2]: https://github.com/primait/event_sourcing.rs/compare/0.6.1...0.6.2
