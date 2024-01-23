
use crate::{Input, Output};
use tracing::error;
use tokio::task::JoinHandle;

pub struct IdentityController {
    pub handle: JoinHandle<()>,
}


impl IdentityController {
    pub fn new<T: Clone + Send + 'static>(
        input: &dyn Input<T>,
        output: &dyn Output<T>,
    ) -> IdentityController {

        let mut source = input.source();
        let sink = output.sink();

        
        let handle = tokio::spawn(async move {

            sink.tx.send(source.start).await.unwrap();

            loop {
                match source.rx.recv().await {
                    Ok(t) => {
                        if let Err(err) = sink.tx.send(t).await {
                            error!("IdentityController error sending to sink: {}", err);
                            break;
                        }
                    },
                    Err(err) => {
                        error!("IdentityController error receiving from source: {}", err);
                        break;
                    }
                }
            }
        });

        IdentityController{
            handle,
        }
    }
}