use std::sync::Arc;

use async_trait::async_trait;
use futures::lock::Mutex;

use esrs::handler::EventHandler;
use esrs::manager::AggregateManager;
use esrs::store::postgres::PgStore;
use esrs::store::StoreEvent;

use crate::aggregate::{SagaAggregate, SagaCommand, SagaEvent};
use crate::SagaError;

#[derive(Clone)]
pub struct SagaEventHandler {
    pub store: PgStore<SagaAggregate>,
    pub side_effect_mutex: Arc<Mutex<bool>>,
}

#[async_trait]
impl EventHandler<SagaAggregate> for SagaEventHandler {
    async fn handle(&self, event: &StoreEvent<SagaEvent>) {
        let manager = AggregateManager::new(self.store.clone());

        if event.payload == SagaEvent::MutationRequested {
            match manager.load(event.aggregate_id).await {
                Ok(Some(state)) => {
                    let mut guard = self.side_effect_mutex.lock().await;
                    *guard = true;
                    if let Err(err) = manager
                        .handle_command::<SagaError>(state, SagaCommand::RegisterMutation)
                        .await
                    {
                        eprintln!("Error while handling register mutation command: {:?}", err)
                    }
                }
                Ok(None) => {
                    eprintln!("Something went wrong getting aggregate state")
                }
                Err(err) => {
                    eprintln!("Failed to perform side effect: {:?}", err)
                }
            }
        }
    }
}
