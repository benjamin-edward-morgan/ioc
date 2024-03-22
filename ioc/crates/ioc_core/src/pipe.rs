//! Includes `Pipe` for reading from `Input`s and writing to `Output`s

use crate::{Input, Output};
use tokio::task::JoinHandle;
use tracing::{debug, error};

///Reads values from the given input, writes them to the given output.
pub struct Pipe {
    pub handle: JoinHandle<()>,
}

impl Pipe {
    ///Create a new `Pipe`. Spawns a task that reads from the input and writes to the output.
    pub fn new<T: Send + Sync + Copy + 'static>(input: &Input<T>, output: &Output<T>) -> Pipe {
        let mut source = input.source();
        let sink = output.sink();

        let handle = tokio::spawn(async move {
            loop {
                let value: T = *source.borrow_and_update();
                if let Err(err) = sink.send(value).await {
                    error!("Pipe error sending to sink: {}", err);
                    return;
                }
                if let Err(err) = source.changed().await {
                    error!("Pipe error receiving from source: {}", err);
                    break;
                }
            }
            debug!("Pipe shutting down!")
        });

        Pipe { handle }
    }
}
