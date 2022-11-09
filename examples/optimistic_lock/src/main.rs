use esrs::*;
use optimistic_lock::*;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let pool = sqlx::PgPool::connect("postgres://postgres:postgres@localhost:5432/postgres")
        .await
        .unwrap();
    let aggregate = LoggingAggregate::new(&pool).await.unwrap();
    let aggregate_two = aggregate.clone();
    let id = Uuid::new_v4();
    println!("Setup..");
    let _ = aggregate
        .handle_command(AggregateState::new(id), LoggingCommand::Log("FIRST".to_string()))
        .await
        .unwrap();
    println!("Setup done!");

    let handle = tokio::spawn(async move {
        println!("ONE - Acquiring lock...");
        let _lock = aggregate.lock(id).await.unwrap();
        println!("ONE - Got the lock! Loading state...");
        let state = aggregate.load(id).await.unwrap();
        println!("ONE - Sleeping for one second...");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        println!("ONE - Handling command...");
        aggregate
            .handle_command(state, LoggingCommand::Log("One".to_string()))
            .await
    });
    let handle_two = tokio::spawn(async move {
        println!("TWO - Acquiring lock...");
        let _lock = aggregate_two.lock(id).await.unwrap();
        println!("TWO - Got the lock! Loading state...");
        let state = aggregate_two.load(id).await.unwrap();
        println!("TWO - Sleeping for one second...");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        println!("TWO - Handling command...");
        aggregate_two
            .handle_command(state, LoggingCommand::Log("Two".to_string()))
            .await
    });

    let (join, join_two) = tokio::join!(handle, handle_two);
    let (result, result_two) = (join.unwrap(), join_two.unwrap());
    println!("ONE - {result:?}");
    println!("TWO - {result_two:?}");
}
