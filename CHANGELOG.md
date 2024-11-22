# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]


- Bump MSRV to `1.81.0`.
- Expose option for setting ID function (#202)
  - Add ID format to `PgStore`
  - Allow `PgStoreBuilder` to set ID format
  - Support UUID version 7 directly

---

## [0.17.1] - 2024-08-20

### Updated

- updated sqlx to 0.8.0

---
## [0.17.0] - 2024-06-10

### Changed

- [[#196]]: `AggregateManager::handle_command` now returns the updated aggregate state, instead of `()`.

---
## [0.16.0] - 2024-05-08

Note: this version contains hard breaking changes in the `AggregateManager` API - refer to [#192] and [#194].

### Added

- [[#194]]: New `LockedLoad` type to correctly manage locked loads in `AggregateManager`.

### Changed

- [[#192]]: `AggregateManager::handle_command` now returns a concrete `Result<Result<(), Aggregate::Error>, Store::Error>` and is no longer generic in error.
- [[#194]]: `AggregateManager::lock_and_load` now returns a `LockedLoad`.

### Fixed

- [[#194]]: Previously, concurrent `lock_and_load`s would drop the lock if the aggregate was empty, leading to concurrent writes (caught by optimistic locking). This is now correctly handled via the `LockedLoad` result.

---
## [0.15.0] - 2024-04-03

### Added

- [[#187]]: Make the `AggregateManager` `deref` blanket implementation work for smart pointers.
- [[#191]]: Add new generic on `PgStore` and `Schema` trait to decouple persistence from `Aggregate::Event`.

### Changed

- [[#191]]: Updated MSRV to `1.74.0`.
- [[#191]]: Renamed `Event` trait to `Persistable` (this should not affect users of the library since users of the library benefit from a blanket implementation).

### Fixed

- [[#189]]: Fixed examples not compiling in Rust `1.75.0`.

### Removed

- [[#191]]: Removed broken `sql` feature.

---
## [0.14.0] - 2024-01-09

### Added

- [[#185]]: `AggregateManager::handle_command` is generic in Error.

### Fixed

- [[#185]]: `PgStore` is now always `Clone`able.

---
## [0.13.0] - 2023-09-12

### Added

- [[#147]]: Event upcasting, available under `upcasting` feature flag, disabled by default.
- [[#164]]: Kafka messages are now published using the record key, set as the event `aggregate_id`.
- [[#175]]: Connection manager for RabbitMQ event bus to handle connection outages and fail-overs.

---
## [0.12.0] - 2023-06-09

### Added 

- [[#148]]: `ReplayableEventHandler` trait to mark an `EventHandler` as replayable or not. This does not stick to
            `TransactionalEventHandlers` since it is taken for granted that they must always be replayable. 
- [[#149]]: `PgStoreBuilder` struct, currently the sole method for constructing a `PgStore`.
- [[#151]]: The `EventBus` trait is integrated with the `PgStore` implementation to facilitate the publishing of events 
            after they have been processed by the handlers.
- [[#152]]: `MigrationHandler` trait to run migrations while building a new `PgStore`.
- [[#155]]: Concrete implementations of the `EventBus` interface for Apache Kafka and RabbitMQ. These implementations 
            are available under the `rabbit` and `kafka` features.
- [[#155]]: Docker compose file for local development and testing.
- [[#156]]: The `table_name` and `add_event_handler` functions to `PgStore`.
- [[#156]]: Generic `Rebuilder` trait and concrete `PgRebuilder` struct facilities to rebuild a single aggregate. These
            implementations are available under the `rebuilder` feature.
- [[#157]]: The `TransactionalEventHandler` now includes a new generic type argument that allow to specify the error 
            return type.
- [[#157]]: The `EventStore` trait now takes the `Aggregate` as associated type and now includes a new associated type 
            that allow to specify the error return type. 
- [[#157]]: New `PgStoreError` type as error return type for `PgStore`.

### Changed

- [[#148]]: `Projector` and `Policy` no longer exists. Replaced with `EventHandler` and `TransactionalEventHandler`. 
- [[#150]]: `AggregateManager` is no longer a trait; it now lives as a struct.
- [[#150]]: The `EventStore`, `PgStore`, `EventHandler`, `TransactionalEventHandler` and `ReplayableEventHandler` types, 
            previously associated with the `AggregateManager` trait, now have a simplified constraint that they are bound
            to the `Aggregate` trait.
- [[#153]]: The `save_event` function in the `PgStore` is now accessible only within the crate scope.
- [[#156]]: The examples have been refactored as external executables and are no longer part of the cargo workspace.
- [[#157]]: The `AggregateManager` type bound has been changed from `Aggregate` to an `EventStore` type.
- [[#157]]: 
- [[#157]]: The return type error of the inner functions in `AggregateManager` has been modified from `Aggregate::Error`
            to a new type called `AggregateManagerError<E>`. This change introduces a clear differentiation between an 
            `Aggregate` error and an `EventStore` error.
- [[#157]]: The functions in the `EventStore`, including those in the `PgStore`, now utilize the new error associated type 
            as their return type.
- [[#161]]: Moved some traits and structs in other packages
  - The `esrs::AggregateManager` struct (previously a trait) moved into `esrs::manager` module.
  - The `esrs::postgres` module has been relocated and can now be found under `esrs::store::postgres`.
  - The `esrs::EventStore`, `esrs::EventStoreLockGuard`, `esrs::StoreEvent` and `esrs::UnlockOnDrop` objects moved to `esrs::store` module.


### Removed

- [[#153]]: `PgStore` getters functions `transactional_event_handlers`, `event_handlers` and `event_buses`.
- [[#153]]: `PgStore` custom `persist` function.
- [[#157]]: The `Clone`, `Send`, and `Sync` bounds on the associated types of `Aggregate` have been eliminated.
- [[#161]]: The `error` module has been removed and is no longer available.

---
## [0.11.0] - 2023-04-03

### Changed

- [[#144]]
  - projector `consistency` function has been renamed in `persistence`.
  - projector `Consistency` enum has been renamed in `ProjectorPersistence`.
  - projector `ProjectorPersistence` enum entries renamed in `Mandatory` and `Fallible`.

---

## [0.10.2] - 2023-02-16

### Changed

- [[#141]]: log error in case of event projection or policy application failure

### Fixed

- [[#141]]: fixed tracing of event projection and policy application

---

## [0.10.1] - 2023-02-06

### Fixed

- [[#136]]: `select_all` query ordering was missing of sequence_number.

---

## [0.10.0] - 2022-11-30

- [[#133]]: atomic read/writes rework to avoid deadlocks in Policies.

### Changed

- `Aggregate`:
  - `apply_events` has been removed, and its default implementation moved to a method of `AggregateState`.
- `AggregateManager`:
  - `lock_and_load` acquires a lock, then loads into memory the AggregateState
    preventing other atomic accesses. Dropping the AggregateState releases the resource.
  - `lock` has been removed in favour of `lock_and_load`.
  - `handle_command` does not return the AggregateState anymore. This avoids race conditions where the
    returned state is already outdated, if another concurrent access has taken place at the same time.
  - `apply_events` has been removed, and its default implementation moved to a method of `AggregateState`.
  - `load` has been changed to return a `Result<Option<_>, _>`, to explicit expose errors.
- `AggregateState`:
  - new `lock` field, which contains the exclusive access guard created when `lock_and_load`ing.
    All other fields are now private for better encapsulation.
  - `new` takes no argument, and generates a new state with a random UUID.
  - `with_id` generates a new state with the given UUID.
  - `apply_store_events` applies the given list of Events on self.
  - `set_lock` and `take_lock` methods to set and get the lock,
    to use when overriding the AggregateManager functionality.

---

## [0.9.0] - 2022-11-21

### Added

- Added tracing spans for application of policies and projection of events

---

## [0.8.0]

### Added

- [[#129]]: Atomic read/writes for aggregate states.
  - `AggregateManager`:
    - `lock` method acquires a lock for the given aggregate state, preventing other atomic accesses.
      Dropping the lock releases the resource.
  - `EventStore`:
    - `lock` trait function, required to return a `EventStoreLockGuard`.
  - `EventStoreLockGuard`, wrapping an `UnlockOnDrop` trait object.
  - `UnlockOnDrop` marker trait, required for concrete types to be used as `EventStoreLockGuard`.
  - `PgStore`:
    - `lock` implementation using Postgres' advisory locks.
  - `PgStoreLockGuard`, holding the actual Postgres' lock and releasing it on drop.

---

## [0.7.1]

### Added

- [[#114]]: Default `EventStore` implementation for every `Box<dyn EventStore<Manager = _>>`. This allows to define
  in the `AggregateManager` the `EventStore` associated type as `Box<dyn EventStore<Manager = _>>` (with `Send` +
  `Sync` bounds).
- [[#115]]: Added `apply_events` to `Aggregate` with default implementation.
- [[#122]]:
  - `Consistency` level enum.
  - `consistency` function to `Projector` to instruct the `EventStore` on the persistence guarantees with default
    implementation returning `Consistency::Strong`.
- [[#123]]: Added `postgres` documentation in docs.rs with `package.metadata.docs.rs` in `Cargo.toml`. Improved
  modules documentation.

# Changed

- [[#117]]:
  - `AggregateState::new` second parameter from `Uuid` to `impl Into<Uuid>`.
  - `AggregateManager::load` first parameter from `Uuid` to `impl Into<Uuid>`.
  - `AggregateState::delete` first parameter from `Uuid` to `impl Into<Uuid>`.
- [[#118]]: Merged rebuild examples into one; removed mains and migrations from examples.

---

## [0.7.0]

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
  - `PgStore` and all its dependencies are now cloneable. Is behind and Arc and is safely cloneable.

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




[Unreleased]: https://github.com/primait/event_sourcing.rs/compare/0.17.1...HEAD
[0.17.1]: https://github.com/primait/event_sourcing.rs/compare/0.17.0...0.17.1
[0.17.0]: https://github.com/primait/event_sourcing.rs/compare/0.16.0...0.17.0
[0.16.0]: https://github.com/primait/event_sourcing.rs/compare/0.15.0...0.16.0
[0.15.0]: https://github.com/primait/event_sourcing.rs/compare/0.14.0...0.15.0
[0.14.0]: https://github.com/primait/event_sourcing.rs/compare/0.13.0...0.14.0
[0.13.0]: https://github.com/primait/event_sourcing.rs/compare/0.12.0...0.13.0
[0.12.0]: https://github.com/primait/event_sourcing.rs/compare/0.11.0...0.12.0
[0.11.0]: https://github.com/primait/event_sourcing.rs/compare/0.10.2...0.11.0
[0.10.2]: https://github.com/primait/event_sourcing.rs/compare/0.10.1...0.10.2
[0.10.1]: https://github.com/primait/event_sourcing.rs/compare/0.10.0...0.10.1
[0.10.0]: https://github.com/primait/event_sourcing.rs/compare/0.9.0...0.10.0
[0.9.0]: https://github.com/primait/event_sourcing.rs/compare/0.8.0...0.9.0
[0.8.0]: https://github.com/primait/event_sourcing.rs/compare/0.7.1...0.8.0
[0.7.1]: https://github.com/primait/event_sourcing.rs/compare/0.7.0...0.7.1
[0.7.0]: https://github.com/primait/event_sourcing.rs/compare/0.6.2...0.7.0
[0.6.2]: https://github.com/primait/event_sourcing.rs/compare/0.6.1...0.6.2

[#196]: https://github.com/primait/event_sourcing.rs/pull/196
[#194]: https://github.com/primait/event_sourcing.rs/pull/194
[#192]: https://github.com/primait/event_sourcing.rs/pull/192
[#191]: https://github.com/primait/event_sourcing.rs/pull/191
[#189]: https://github.com/primait/event_sourcing.rs/pull/189
[#187]: https://github.com/primait/event_sourcing.rs/pull/187
[#185]: https://github.com/primait/event_sourcing.rs/pull/185
[#175]: https://github.com/primait/event_sourcing.rs/pull/175
[#164]: https://github.com/primait/event_sourcing.rs/pull/164
[#161]: https://github.com/primait/event_sourcing.rs/pull/161
[#157]: https://github.com/primait/event_sourcing.rs/pull/157
[#156]: https://github.com/primait/event_sourcing.rs/pull/156
[#155]: https://github.com/primait/event_sourcing.rs/pull/155
[#153]: https://github.com/primait/event_sourcing.rs/pull/153
[#152]: https://github.com/primait/event_sourcing.rs/pull/152
[#151]: https://github.com/primait/event_sourcing.rs/pull/151
[#150]: https://github.com/primait/event_sourcing.rs/pull/150
[#149]: https://github.com/primait/event_sourcing.rs/pull/149
[#148]: https://github.com/primait/event_sourcing.rs/pull/148
[#147]: https://github.com/primait/event_sourcing.rs/pull/147
[#144]: https://github.com/primait/event_sourcing.rs/pull/144
[#141]: https://github.com/primait/event_sourcing.rs/pull/141
[#136]: https://github.com/primait/event_sourcing.rs/pull/136
[#133]: https://github.com/primait/event_sourcing.rs/pull/133
[#129]: https://github.com/primait/event_sourcing.rs/pull/129
[#123]: https://github.com/primait/event_sourcing.rs/pull/123
[#122]: https://github.com/primait/event_sourcing.rs/pull/122
[#118]: https://github.com/primait/event_sourcing.rs/pull/118
[#117]: https://github.com/primait/event_sourcing.rs/pull/117
[#115]: https://github.com/primait/event_sourcing.rs/pull/115
[#114]: https://github.com/primait/event_sourcing.rs/pull/114
[#109]: https://github.com/primait/event_sourcing.rs/pull/109
[#108]: https://github.com/primait/event_sourcing.rs/pull/108
[#107]: https://github.com/primait/event_sourcing.rs/pull/107
