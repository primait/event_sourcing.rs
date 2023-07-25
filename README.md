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

## Usage

In this section is presented the integration of this library into your application, enabling you to kickstart 
CQRS/Event sourcing implementation.

### Event sourcing

> Event sourcing is a software architectural pattern used to capture and store the changes or events that occur in an 
application's state over time. Instead of persisting the current state of an object or entity, event sourcing focuses 
on recording a sequence of events that have led to that state.

#### Aggregate

To initiate the implementation of CQRS/Event sourcing in your codebase, the initial building block to create is an `Aggregate`.

> In the context of event sourcing, an aggregate is a fundamental concept that represents a cluster of domain objects 
and their state changes. It is one of the key building blocks used to model and manage data in an event-sourced system.

> An aggregate, therefore, is responsible for processing incoming commands and generating corresponding events that 
reflect the changes to its state. It encapsulates domain logic and rules related to specific operations on the data it 
manages. Aggregates provide a clear boundary for consistency and concurrency control within the event-sourced system.

Implement your own aggregate:

```rust
pub struct Book;

impl Aggregate for Book {
    ...
}
```

Each implementation of the `Aggregate` trait is required to define specific types and functions that fulfill its interface:

- `const NAME`: constant value. It refers to the unique identifier or key that is used to identify a specific aggregate 
instance within the system. 
- `type State`: refers to the current state of a specific aggregate instance at a given point in time.  It represents the 
collective data and attributes that define the entity represented by the aggregate.
- `type Command`: a request or a message that represents an intention to perform a specific action or operation on an 
aggregate instance.
- `type Event`: is a representation of a significant state change or an occurrence that has happened to a specific aggregate
instance. Aggregate events are at the core of event sourcing as they capture all the changes made to an aggregate over time.
- `type Error`: is a representation of every validation failure that can occur while trying to handle a command.

- `fn handle_command`: is responsible for processing and validating incoming commands and emitting corresponding events.
- `fn apply_event`: this function is responsible for processing individual events, or replaying batch of events, and 
applying their effects on the `Aggregate`'s state.

Here some example implementations:

`NAME`:

```rust
const NAME: &'static str = "book";
```

`State`

```rust
pub struct BookState {
    pub leftover: i32,
}

impl Default for BookState {
    fn default() -> Self {
        Self { leftover: 10 }
    }
}
```

`Command`

```rust
pub enum BookCommand {
    Buy {
        num_of_copies: i32,
    },
    Return {
        num_of_copies: i32,
    },
}
```

`Event`

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub enum BookEvent {
    Bought {
        num_of_copies: i32,
    },
    Returned {
        num_of_copies: i32,
    }
}
```

`Error`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BookError {
    NotEnoughCopies,
}
```

And now let's put all together in the `Aggregate`, implementing `handle_command` and `apply_events` functions too.

```rust
...

impl Aggregate for Book {
    const NAME: &'static str = "book";
    type State = BookState;
    type Command = BookCommand;
    type Event = BookEvent;
    type Error = BookError;

    fn handle_command(state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            BookCommand::Buy { num_of_copies } if state.leftover < num_of_copies => Err(BookError::NotEnoughCopies),
            BookCommand::Buy { num_of_copies } => Ok(vec![BookEvent::Bought { num_of_copies }]),
            BookCommand::Return { num_of_copies } => Ok(vec![BookEvent::Returned { num_of_copies }]),
        }
    }

    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State {
        match payload {
            BookEvent::Bought { num_of_copies } => BookState { leftover: state.leftover - num_of_copies },
            BookEvent::Returned { num_of_copies } => BookState { leftover: state.leftover + num_of_copies },
        }
    }
}
```

#### Adding a persistence layer

At the time being the only existing persistence layer is the `PgStore`. Its role is to write all the events in a dedicated
table in `postgres`. The table name is built using `Aggregate::NAME` constant value concatenated with `_events`.

!Important: each `PgStore` is designed to be globally unique per Aggregate. If you require constructing it dynamically 
at runtime, remember to invoke `without_running_migrations` while building the store to avoid executing migrations.

Note: In order to use a `PgStore` it's needed a fully implemented `Aggregate`.

```rust
use sqlx::{Pool, Postgres};

let pool: Pool<Postgres> = unimplemented!();

// Building a `PgStore`
let store: PgStore<Book> = PgStoreBuilder::new(pool)
        .try_build()
        .await
        .expect("Failed to create PgStore");

let mut state: BookState = BookState::default();
// Using the store
let events: Vec<BookEvent> = Book::handle_command(&state, BookCommand::Buy { num_of_copies: 1 });

store.persist(&mut state, events).await?;
```

To alleviate the burden of writing all of this code, you can leverage the `AggregateManager` helper. An `AggregateManager`
could be considered as a synchronous `CommandBus`.

```rust
let manager: AggregateManager<_> = AggregateManager::new(store);
manager.handle_command(Default::default(), BookCommand::Buy { num_of_copies: 1 }).await
```

### CQRS

> CQRS stands for Command Query Responsibility Segregation, which is a software architectural pattern that segregates the 
responsibility for handling read and write operations in an application. The core idea behind CQRS is to split the 
application's data model into two distinct models: one optimized for read operations (the Query side) and another for 
write operations (the Command side).

#### The sync way

In the purist concept of CQRS, this approach might provoke debates. Nevertheless, the underlying intent is to provide 
timely feedback to users upon completion of operations, considering various scenarios.

For instance, it aims to eliminate the need for a hypothetical frontend to continually poll the backend for updates, by 
ensuring the read side is promptly updated. Additionally, it caters to scenarios where a sequence of operations forms a 
chain, and completing the entire task is crucial, as in the case of a SAGA pattern.

Overall, the primary goal is to enhance user experience and system efficiency by employing well-defined and specialized 
models for read and write operations, tailoring the architecture to specific use cases.

The two main pillars in the context of CQRS/Event sourcing are "Event Handlers" and "Transactional Event Handlers."

##### Event handlers

An `EventHandler`, by definition, operates on an eventually consistent basis and primarily serves to update the read side 
of the application. It is also commonly utilized to execute commands on other aggregates, including the aggregate to 
which it belongs.

`EventHandler`s are infallible.

```rust
pub struct BookEventHandler;

#[async_trait::async_trait]
impl EventHandler<Book> for BookEventHandler {
    async fn handle(&self, event: &StoreEvent<BookEvent>) {
        // Implementation here
    }
}

// Where the store is built..
PgStoreBuilder::new(pool)
    // ..add your event handler
    .add_event_handler(BookEventHandler)
    .try_build()
    .await
    .expect("Failed to create PgStore");
```

##### Transactional event handlers

`TransactionalEventHandlers` are a specialized form of Event Handlers that are designed to maintain transactional 
consistency between the write and read models. 

If a failure occurs in a `TransactionalEventHandlers`, it triggers a complete rollback of the transaction and 
subsequently returns an error to the caller.

!Important: using a `TransactionalEventHandler` to execute commands on another aggregate is strongly discouraged.

```rust
pub struct BookTransactionalEventHandler;

#[async_trait::async_trait]
impl TransactionalEventHandler<Book, PgStoreError, PgConnection> for BookTransactionalEventHandler {
    async fn handle(&self, event: &StoreEvent<BookEvent>, transaction: &mut PgConnection) -> Result<(), PgStoreError> {
        // Implementation here
        Ok(())
    }
}

// Where the store is built..
PgStoreBuilder::new(pool)
    // ..add your transactional event handler
    .add_transactional_event_handler(BookTransactionalEventHandler)
    .try_build()
    .await
    .expect("Failed to create PgStore");
```

#### The async way

This approach adheres to the more traditional principles of CQRS. When events are emitted from the write side of the 
application, the corresponding event handlers process them asynchronously. This approach enhances responsiveness and 
scalability by allowing the application to handle events concurrently.

##### Event bus

While right now there are two different implementations of event bus, one using rabbit and the other using kafka, it's possible to 
have a custom implementation of an event bus. To do so just implement the `EventBus` trait.

For the sake of this example we create a strongly coupled event bus while a more generic implementation is preferable.

```rust
pub struct BookEventBus;

#[async_trait::async_trait]
impl EventBus<Book> for BookEventBus {
    async fn publish(&self, store_event: &StoreEvent<BookEvent>) {
        // Implementation herepub struct BookEventBus;

#[async_trait::async_trait]
impl EventBus<Book> for BookEventBus {
    async fn publish(&self, store_event: &StoreEvent<BooKEvent>) {
        // Implementation herre
    }
}
    }
}
```

Subsequently, integrate the `EventBus` into the `PgStore` at build time.

```rust
// Where the store is built..
PgStoreBuilder::new(pool)
    // ..add your event handler
    .add_event_bus(BookEventBus)
    .try_build()
    .await
    .expect("Failed to create PgStore");
```

##### Consuming the bus and the event handler ... again!

The subsequent phase to finalize the asynchronous approach is to construct a read side. To accomplish this, we continue 
to rely on the `EventHandler`. However, the library lacks a built-in consumer. Implementing the consumer is left to the 
discretion of the library user. The concept behind this "consumer" is to consume messages from the bus and then apply
them to each `EventHandler` responsible for carrying out specific tasks.

As the implementation closely resembles the "sync way" approach, we avoid to write it again.

### Follow up

You can find a comprehensive example, incorporating all the presented contents from the [Usage](#usage) section, 
in the [readme example](./examples/readme/main.rs).