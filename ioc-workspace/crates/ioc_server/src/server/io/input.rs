use crate::server::state::{StateUpdate,ServerInputState};

use ioc_core::{Input, InputSource};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::log::{error, warn};

pub struct ServerInput<T: Clone + Send + 'static> {
    pub handle: JoinHandle<()>,
    value: Arc<Mutex<T>>,
    rx: broadcast::Receiver<T>,
}

impl<T: Clone + Send + 'static> ServerInput<T> {
    pub(crate) fn new(
        key: String,
        channel_size: usize,
        start: T,
        mut state_bcast: broadcast::Receiver<StateUpdate>,
        transform: fn(&ServerInputState) -> Option<T>,
    ) -> Self {
        let value = Arc::new(Mutex::new(start.clone()));
        let (tx, rx) = broadcast::channel(channel_size);
        let handle = tokio::spawn(async move {
            loop {
                match state_bcast.recv().await {
                    Ok(update) => {
                        if let Some(new_value) = update.inputs.get(&key) {
                            if let Some(transformed) = transform(new_value) {
                                if let Err(_err) = tx.send(transformed) {
                                    error!("server inbput {:?} shutting down because of error sending updated broadcast value", key);
                                    break;
                                }
                            }
                            
                        }
                    }
                    Err(err) => {
                        warn!("server input {:?} shutting down because {}", key, err);
                        break;
                    }
                }
            }
        });
        ServerInput { handle, value, rx }
    }
}

impl<T: Clone + Send + 'static> Input<T> for ServerInput<T> {
    fn source(&self) -> InputSource<T> {
        let guard = match self.value.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        InputSource {
            start: guard.clone(),
            rx: self.rx.resubscribe(),
        }
    }
}
