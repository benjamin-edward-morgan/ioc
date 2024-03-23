use std::collections::HashMap;

use ioc_core::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use tokio::task::JoinHandle;
use tracing::{debug, warn};


pub struct LinearTransformConfig<'a> {
    pub input: &'a Input<f64>,
    pub m: f64,
    pub b: f64,
}

impl LinearTransformConfig<'_> {
    pub fn from_ranges<'a>(
        input: &'a Input<f64>,
        from_range: &[f64; 2],
        to_range: &[f64; 2],
    ) -> Result<LinearTransformConfig<'a>, IocBuildError> {
        if from_range[0] == from_range[1] {
            return Err(IocBuildError::message(
                "can't build linear transform where from_range has no width",
            ));
        }
        let m = (to_range[1] - to_range[0]) / (from_range[1] - from_range[0]);
        let b = to_range[0] - from_range[0] * m;
        Ok(LinearTransformConfig { input, m, b })
    }
}

pub struct LinearTransform {
    pub join_handle: JoinHandle<()>,
    pub value: Input<f64>,
}

impl From<LinearTransform> for TransformerI {
    fn from(ltrans: LinearTransform) -> Self {
        TransformerI {
            join_handle: ltrans.join_handle,
            inputs: HashMap::from([("value".to_owned(), InputKind::Float(ltrans.value))]),
        }
    }
}

impl<'a> Transformer<'a> for LinearTransform {
    type Config = LinearTransformConfig<'a>;

    async fn try_build(cfg: &Self::Config) -> Result<Self, IocBuildError> {
        let mut in_rx = cfg.input.source();
        let start = *in_rx.borrow_and_update();
        let start = start * cfg.m + cfg.b;
        let (value, out_tx) = Input::new(start);
        let m = cfg.m;
        let b = cfg.b;
        let handle = tokio::spawn(async move {
            while in_rx.changed().await.is_ok() {
                let new_input = *in_rx.borrow_and_update();
                let new_output = new_input * m + b;
                if let Err(err) = out_tx.send(new_output) {
                    warn!("Error ending output! {:?}", err);
                    break;
                }
            }
            debug!("shutting down linear transformer!");
        });

        Ok(LinearTransform {
            join_handle: handle,
            value,
        })
    }
}
