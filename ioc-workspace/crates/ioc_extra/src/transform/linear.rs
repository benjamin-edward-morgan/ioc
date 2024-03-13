use std::collections::HashMap;

use ioc_core::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use tokio::{sync::broadcast, task::JoinHandle};
use tracing::warn;

use crate::input::SimpleInput;

pub struct LinearTransformConfig<'a> {
    pub input: &'a dyn Input<f64>,
    pub m: f64,
    pub b: f64,
}

impl LinearTransformConfig<'_> {
    pub fn from_ranges<'a>(
        input: &'a dyn Input<f64>,
        from_range: &[f64 ; 2],
        to_range: &[f64 ; 2],   
    ) -> Result<LinearTransformConfig<'a>, IocBuildError> {
        if from_range[0] == from_range[1] {
            return Err(
                IocBuildError::message("can't build linear transform where from_range has no width")
            );
        }
        let m = (to_range[1] - to_range[0])/(from_range[1] - from_range[0]);
        let b = to_range[0] - from_range[0]*m;
        Ok(LinearTransformConfig{
            input,
            m,
            b
        })
    }
}

pub struct LinearTransform {
    pub join_handle: JoinHandle<()>,
    pub value: SimpleInput<f64>,
}

impl From<LinearTransform> for TransformerI {
    fn from(ltrans: LinearTransform) -> Self {
        TransformerI{
            join_handle: ltrans.join_handle,
            inputs: HashMap::from([
                ("value".to_owned(), InputKind::float(ltrans.value))
            ])
        }
    }
}

impl <'a> Transformer<'a> for LinearTransform {
    type Config = LinearTransformConfig<'a>;
    
    async fn try_build(cfg: &Self::Config) -> Result<Self, IocBuildError>  {
        let in_src = cfg.input.source();
        let mut in_rx = in_src.rx;
        let (out_tx, out_rx) = broadcast::channel(10);
        let start = in_src.start * cfg.m + cfg.b;
        let value = SimpleInput::new(start, out_rx);
        let m = cfg.m;
        let b = cfg.b;
        let handle = tokio::spawn(async move {
            while let Ok(new_input) = in_rx.recv().await {
                let new_output = new_input * m + b;
                if let Err(err) = out_tx.send(new_output) {
                    warn!("Error ending output! {:?}", err);
                    break;
                }
            }
        });

        Ok(LinearTransform{
            join_handle: handle,
            value
        })
    }
}