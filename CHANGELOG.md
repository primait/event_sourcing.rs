# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Aggregate` should implement `name` function that act as `Identifier`.
- `AggregateManager` should implement `EventStore` associated type.

### Changed

- `Aggregate` is now pure. API changed so user have to implement `Aggregate` for logic and `AggregateManager` in 
  order to handle persistence layer.
- `PgProjector` and `PgProjectorEraser` second parameter changed from `PoolConnection` to `Transaction`.
- `Aggregate::apply_event` `payload` parameter changed from reference to value (`Self::Event`).
- `AggregateManager::event_store` changed to return a reference to it's associated type `EventStore`.

### Removed

- `Aggregate::validate_command` is removed; now validation should be made in `handle_command`.
- `Sqlite` feature and its implementation.
- `Identifier` trait.

## [0.6.2]

### Changed

- Bump min version of supported Rust to 1.58 since <1.58 fails to resolve sqlx-core dep

[Unreleased]: https://github.com/primait/event_sourcing.rs/compare/0.6.2...HEAD
[0.6.2]: https://github.com/primait/event_sourcing.rs/compare/0.6.1...0.6.2
