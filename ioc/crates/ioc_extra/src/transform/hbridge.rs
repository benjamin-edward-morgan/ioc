use std::collections::HashMap;

use ioc_core::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use tokio::task::JoinHandle;
use tracing::debug;

pub struct HBridgeConfig<'a> {
    pub input: &'a Input<f64>,
}

pub struct HBridge {
    pub join_handle: JoinHandle<()>,
    pub forward: Input<f64>,
    pub reverse: Input<f64>,
    pub enable: Input<f64>,
}

impl From<HBridge> for TransformerI {
    fn from(hbridge: HBridge) -> Self {
        TransformerI {
            join_handle: hbridge.join_handle,
            inputs: HashMap::from([
                ("forward".to_owned(), InputKind::Float(hbridge.forward)),
                ("reverse".to_owned(), InputKind::Float(hbridge.reverse)),
                ("enable".to_owned(), InputKind::Float(hbridge.enable)),
            ]),
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

impl<'a> Transformer<'a> for HBridge {
    type Config = HBridgeConfig<'a>;

    async fn try_build(cfg: &HBridgeConfig<'a>) -> Result<HBridge, IocBuildError> {
        let mut in_rx = cfg.input.source();
        let (fwd, rev, en) = hbridge_outputs(*in_rx.borrow_and_update());

        let (forward, fwd_tx) = Input::new(fwd);
        let (reverse, rev_tx) = Input::new(rev);
        let (enable, en_tx) = Input::new(en);

        let join_handle = tokio::spawn(async move {
            while in_rx.changed().await.is_ok() {
                let (fwd, rev, en) = hbridge_outputs(*in_rx.borrow_and_update());
                fwd_tx.send(fwd).expect("failed to send hbridge value");
                rev_tx.send(rev).expect("failed to send hbridge value");
                en_tx.send(en).expect("failed to send hbridge value");
            }
            debug!("hbridge shutting down!")
        });

        Ok(HBridge {
            join_handle,
            forward,
            reverse,
            enable,
        })
    }
}
