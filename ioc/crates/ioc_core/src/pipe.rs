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
    pub fn new<T: Clone + Send + 'static>(input: &dyn Input<T>, output: &dyn Output<T>) -> Pipe {
        let mut source = input.source();
        let sink = output.sink();

        let handle = tokio::spawn(async move {
            sink.tx.send(source.start).await.unwrap();

            loop {
                match source.rx.recv().await {
                    Ok(t) => {
                        if let Err(err) = sink.tx.send(t).await {
                            error!("Pipe error sending to sink: {}", err);
                            break;
                        }
                    }
                    Err(err) => {
                        error!("Pipe error receiving from source: {}", err);
                        break;
                    }
                }
            }
            debug!("Pipe shutting down!")
        });

        Pipe { handle }
    }
}
