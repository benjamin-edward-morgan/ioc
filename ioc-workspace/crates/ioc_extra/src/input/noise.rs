use std::{collections::HashMap, time::Duration};
use ioc_core::{error::IocBuildError, InputKind, Module, ModuleIO};
use serde::Deserialize;
use tokio::{sync::broadcast, task::JoinHandle, time::sleep};
use rand::random;
use tracing::warn;

use super::SimpleInput;

#[derive(Deserialize, Debug)]
pub struct NoiseInputConfig {
    min: f64,
    max: f64,
    period_ms: u64
}

pub struct NoiseInput {
    pub handle: JoinHandle<()>,
    pub value: SimpleInput<f64>,
}

impl From<NoiseInput> for ModuleIO {
    fn from(noise: NoiseInput) -> Self {
        ModuleIO { 
            join_handle: noise.handle, 
            inputs: HashMap::from([("value".to_owned(), InputKind::float(noise.value))]), 
            outputs: HashMap::new()
        }
    }
}

impl Module for NoiseInput {
    type Config = NoiseInputConfig;

    async fn try_build(cfg: &NoiseInputConfig) -> Result<NoiseInput, IocBuildError>  {
        if cfg.max < cfg.min {
            Err(
                IocBuildError::message("Must have max >= min for Noise input.")
            )
        } else {
            let (tx, rx) = broadcast::channel(10);
            let m = cfg.max - cfg.min;
            let b = cfg.min;
            let current_value = random::<f64>()*m+b;
            let input = SimpleInput::new(current_value, rx);
            let period_ms = cfg.period_ms;
            let handle = tokio::spawn(async move {
                sleep(Duration::from_millis(period_ms)).await;
                loop{
                    let new_value = random::<f64>()*m+b;

                    if let Err(err) = tx.send(new_value) {
                        warn!("error sending value from noise. shutting down! {:?}", err);
                        break;
                    }

                    sleep(Duration::from_millis(period_ms)).await;
                }
            }); 

            Ok(
                NoiseInput { handle, value: input }
            )
        }
    }

}
