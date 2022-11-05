use optimistic_lock::*;
use esrs::*;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let pool = sqlx::PgPool::connect("postgres://postgres:postgres@localhost:5432/postgres").await.unwrap();
    let aggregate = LoggingAggregate::new(&pool).await.unwrap();
    let aggregate_two = aggregate.clone();
    let id = Uuid::new_v4();
    let _ = aggregate.handle_command(AggregateState::new(id), LoggingCommand::Log("FIRST".to_string())).await.unwrap();

    let handle = tokio::spawn(async move {
        let state = aggregate.load(id).await.unwrap();
        aggregate.handle_command(state, LoggingCommand::Log("One".to_string())).await
    });
    let handle_two = tokio::spawn(async move {
        let state = aggregate_two.load(id).await.unwrap();
        aggregate_two.handle_command(state, LoggingCommand::Log("Two".to_string())).await
    });

    let (join, join_two) = tokio::join!(handle, handle_two);
    let (result, result_two) = (join.unwrap(), join_two.unwrap());
    println!("ONE - {result:?}");
    println!("TWO - {result_two:?}");
}
