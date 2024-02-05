
use ioc_core::{Input, Output};
use tracing::{info,error};
use tokio::task::JoinHandle;


pub struct HBridgeController {
    pub handle: JoinHandle<()>,
}

impl HBridgeController {
    pub async fn new(
        input: &dyn Input<f64>,
        fwd: &dyn Output<f64>,
        rev: &dyn Output<f64>,
        enable: &dyn Output<f64>,
    ) -> Self {

        let mut source = input.source();
        let fwd = fwd.sink();
        let rev = rev.sink();
        let enable = enable.sink();

        if source.start > 0.0 {
            //clamp to 1.0
            let f = source.start.min(1.0);
            info!("hbridge-f: {}", f);
            fwd.tx.send(f).await.unwrap();
            rev.tx.send(0.0).await.unwrap();
            enable.tx.send(1.0).await.unwrap();
        } else if source.start < 0.0 {
            let r = (source.start * -1.0).min(1.0);
            info!("hbridge-r: {}", r);
            rev.tx.send(r).await.unwrap();
            fwd.tx.send(0.0).await.unwrap();
            enable.tx.send(1.0).await.unwrap();
        } else {
            rev.tx.send(0.0).await.unwrap();
            fwd.tx.send(0.0).await.unwrap();
            enable.tx.send(0.0).await.unwrap();
        }
        

        let handle = tokio::spawn(async move {
            loop {
                match source.rx.recv().await {
                    Ok(i) => {
                        if i > 0.0 {
                            //clamp to 1.0
                            let f = i.min(1.0);
                            fwd.tx.send(f).await.unwrap();
                            rev.tx.send(0.0).await.unwrap();
                            enable.tx.send(1.0).await.unwrap();
                        } else if i < 0.0 {
                            //flip sign, clamp to 1.0
                            let r = (i * -1.0).min(1.0);
                            rev.tx.send(r).await.unwrap();
                            fwd.tx.send(0.0).await.unwrap();
                            enable.tx.send(1.0).await.unwrap();
                        } else {
                            rev.tx.send(0.0).await.unwrap();
                            fwd.tx.send(0.0).await.unwrap();
                            enable.tx.send(0.0).await.unwrap();
                        }
                    },
                    Err(err) => {
                        error!("Got receive error in h-bridge controller! {:?}", err);
                        //break;
                    }
                }
            }
            error!("hbridge controller shut down!");
        });

        Self{ handle }
    }
}