use std::sync::{Arc, Mutex};

use ioc_core::{Input, InputSource, Output, OutputSink};
use tokio::sync::{broadcast, mpsc};

pub mod noise;

pub struct SimpleOutput<T> {
    pub tx: mpsc::Sender<T>,
}

impl<T> Output<T> for SimpleOutput<T> {
    fn sink(&self) -> OutputSink<T> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}

pub struct SimpleInput<T: Clone> {
    current_value: Arc<Mutex<T>>,
    rx: broadcast::Receiver<T>,
}

impl<T: Clone> SimpleInput<T> {
    pub fn new(start: T, rx: broadcast::Receiver<T>) -> Self {
        //todo: subscribe to rx and update current_value

        SimpleInput {
            current_value: Arc::new(Mutex::new(start)),
            rx,
        }
    }
}

impl<T: Clone> Input<T> for SimpleInput<T> {
    fn source(&self) -> InputSource<T> {
        let start: T = match self.current_value.lock() {
            Ok(current_value) => (*current_value).clone(),
            Err(mut poisoned) => poisoned.get_mut().clone(),
        };

        InputSource {
            start,
            rx: self.rx.resubscribe(),
        }
    }
}
