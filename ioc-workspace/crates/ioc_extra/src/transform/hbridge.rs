use std::collections::HashMap;

use ioc_core::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use tokio::{sync::broadcast, task::JoinHandle};

use crate::input::SimpleInput;

pub struct HBridgeConfig<'a> {
    pub input: &'a dyn Input<f64>,
}

pub struct HBridge {
    pub join_handle: JoinHandle<()>,
    pub forward: SimpleInput<f64>,
    pub reverse: SimpleInput<f64>,
    pub enable: SimpleInput<f64>,
}

impl From<HBridge> for TransformerI {
    fn from(hbridge: HBridge) -> Self {
        TransformerI{
            join_handle: hbridge.join_handle,
            inputs: HashMap::from([
                ("forward".to_owned(), InputKind::float(hbridge.forward)),
                ("reverse".to_owned(), InputKind::float(hbridge.reverse)),
                ("enable".to_owned(), InputKind::float(hbridge.enable)),      
            ])
        }
    }
}

fn hbridge_outputs(input: f64) -> (f64, f64, f64) {
    if input > 0.0 {
        (input, 0.0, 1.0)
    } else if input < 0.0 {
        (0.0, -input, 1.0)
    } else {
        (0.0, 0.0, 0.0)
    }
}

impl <'a> Transformer<'a> for HBridge {
    type Config = HBridgeConfig<'a>;

    async fn try_build(cfg: &HBridgeConfig<'a>) -> Result<HBridge, IocBuildError> {

        let in_src = cfg.input.source();
        let mut in_rx = in_src.rx;
        let (fwd, rev, en) = hbridge_outputs(in_src.start);
        let (fwd_tx, fwd_rx) = broadcast::channel(10);
        let (rev_tx, rev_rx) = broadcast::channel(10);
        let (en_tx, en_rx) = broadcast::channel(10);
        
        let join_handle = tokio::spawn(async move {
            while let Ok(input) = in_rx.recv().await {
                let (fwd, rev, en) = hbridge_outputs(input);
                fwd_tx.send(fwd).expect("failed to send hbridge value");
                rev_tx.send(rev).expect("failed to send hbridge value");
                en_tx.send(en).expect("failed to send hbridge value");
            }
        });
        
        Ok(HBridge { 
            join_handle,
            forward: SimpleInput::new(fwd, fwd_rx), 
            reverse: SimpleInput::new(rev, rev_rx), 
            enable: SimpleInput::new(en, en_rx) 
        })
    }
}