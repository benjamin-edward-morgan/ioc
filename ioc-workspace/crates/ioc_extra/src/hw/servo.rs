
use ioc_core::{Input, Output};
use tracing::{info,error};
use tokio::task::JoinHandle;


pub struct ServoController {
    pub handle: JoinHandle<()>,
}

impl ServoController {
    pub async fn new(
        input: &dyn Input<f64>,
        output: &dyn Output<f64>,
    ) -> Self {

        let mut source = input.source();
        let sink = output.sink();
    
        sink.tx.send(0.0).await.unwrap();
        
        let min = 0.05;
        let max = 0.15;

        let handle = tokio::spawn(async move {
            loop {
                match source.rx.recv().await {
                    Ok(i) => {
                        //[-1,1] => [min, max]
                        let x = min+(max-min)*(i + 1.0)/2.0;
                        sink.tx.send(x).await.unwrap();
                    },
                    Err(err) => {
                        error!("Got receive error in servo controller! {:?}", err);
                        //break;
                    }
                }
            }
        });

        Self{ handle }
    }
}