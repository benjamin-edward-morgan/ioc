
use crate::{Input, Output};
use tracing::{info,error};
use tokio::task::JoinHandle;
pub struct Pipe {
    pub handle: JoinHandle<()>,
}

impl Pipe {
    pub fn new<T: Clone + Send + 'static>(
        input: &dyn Input<T>,
        output: &dyn Output<T>,
    ) -> Pipe {

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
                    },
                    Err(err) => {
                        error!("Pipe error receiving from source: {}", err);
                    }
                }
            }
            info!("Pipe shutting down!")
        });

        Pipe{
            handle,
        }
    }
}