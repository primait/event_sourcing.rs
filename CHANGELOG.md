# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Note: this version contains hard breaking changes and may take a lot of time in order to upgrade library version!
Refer to: [#107], [#108] and [#109]

### Added

- `AggregateManager` 
  - should implement `name` function that act as `Identifier`. Be sure to not change the name previously set in 
    `Identifier::name` function. This would cause the store to create a new table, losing pre-migration events.
  - depends on `Aggregate`, so user must implement `Aggregate` trait in order to implement `AggregateManager` trait.
  - should implement `EventStore` associated type.

- `EventStore::delete` function with which an entire aggregate could be deleted by `aggregate_id`. 

- `PgStore` 
  - `setup` function to create table and indexes if not exists. This function should be used only once at your 
    application startup. It tries to create the event table and its indexes if they not exist.
  - `set_projectors` function to set the store projectors list.
  - `set_policies` function to set the store policies list.
  - `PgStore` and all its dependencies are now cloneable.

- `Projector` should implement `delete` function.

### Changed

- Aliases of exposed traits and struct are hardly changed. Now most of internal objects are flatten in `esrs` module.

- `Aggregate` 
  - is now pure. API changed so user have to implement `Aggregate` for logic and `AggregateManager` in 
    order to handle persistence layer.
  - `handle_command` state argument changed from `&AggregateState<Self::State>` to `&Self::State`.
  - `apply_event` `payload` parameter changed from reference to value (`Self::Event`).

- `AggregateManager`
  - is now dependent by `Aggregate` and no default implementation is provided. To complete the migration for an 
    aggregate handling the persistence layer is now mandatory for your type to implement `AggregateManager`.
  - renamed function `handle` in `handle_command`.
  - `event_store` changed to return a reference to it's associated type `EventStore`.

- `EventStore`
  - `Event` and `Error` generics removed in favour of `Manager: AggregateManager` associated type.

- `PgStore`
  - `new` function is now sync and its return value is no longer a `Result` but `Self`. Removed `Aggregate` type param.
  - `new` takes ownership of pool; removed projectors and policies params. Use `set_projectors` or `set_policies` 
    instead to add them to the store.
  - `rebuild_events` renamed into `stream_events`. Now it takes an `sqlx::Executor` parameter.
  - policies behaviour is now that if one of them fails they fail silently. (override this behaviour with 
    `Aggregate::store_events` using `EventStore::persist` function).
  - `Event` and `Error` trait generic params removed in favour of `Manager: AggregateManager`.
  - `projectors` and `policies` returns an `Arc` to value.

- `PgProjector`
  - renamed to `Projector`.
  - second param changed from `&mut PoolConnection<Postgres>` to `&mut PgConnection`.
  - `Event` and `Error` trait generic params removed in favour of `Manager: AggregateManager`.

- `PgPolicy` 
  - renamed to `Policy`.
  - `Event` and `Error` trait generic params removed in favour of `Manager: AggregateManager`.
  - moved to `esrs` root module.
  - removed second param (`&Pool<Postgres>`).

- `Policy::handle_event` does not have `Pool<Postgres` anymore as param. Executor should be put in the policy at 
  instantiation time.

### Removed

- `sqlite` feature and its implementation.

- `Aggregate`
  - `validate_command` is removed; now validation should be made in `handle_command`.
  - `event_store` function is moved from `Aggregate` to `AggregateManager`.

- `EventStore`
  - `run_policies`. To customize the way policies behave override `Aggregate::store_events` using 
    `EventStore::persist` function.
  - `close` function.

- `PgStore`
  - `test` function. Use `#[sqlx::test]` in your tests to test the store.
  - `begin`, `commit` and `rollback` functions.
  - `Event` generic type.
  - `Error` generic type.
  - `Projector` generic type.
  - `Policy` generic type.

- `Identifier` trait.
- `Eraser` trait.
- `PgProjectorEraser` trait.
- `EraserStore` trait.
- `ProjectorStore` trait.
- `StoreEvent` bounds of `Event` generic.

- `AggregateState`
  - `new_with_state` removed due to potential inconsistency while loading state.


---

## [0.6.2]

### Changed

- Bump min version of supported Rust to 1.58 since <1.58 fails to resolve sqlx-core dep

[Unreleased]: https://github.com/primait/event_sourcing.rs/compare/0.6.2...HEAD
[0.6.2]: https://github.com/primait/event_sourcing.rs/compare/0.6.1...0.6.2

[#107]: https://github.com/primait/event_sourcing.rs/pull/107
[#108]: https://github.com/primait/event_sourcing.rs/pull/108
[#109]: https://github.com/primait/event_sourcing.rs/pull/109
