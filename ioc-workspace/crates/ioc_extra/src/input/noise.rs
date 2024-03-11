use std::{collections::HashMap, time::Duration};

use ioc_core::{InputKind, ModuleIO, ModuleBuilder, OutputKind};
use serde::Deserialize;
use tokio::{sync::broadcast, task::JoinHandle, time::sleep};
use rand::random;
use tracing::warn;

use super::{InputConfigError, SimpleInput};

#[derive(Deserialize, Debug)]
pub struct NoiseInputConfig {
    min: f64,
    max: f64,
    period_ms: u64
}

pub struct NoiseInput {
    pub handle: JoinHandle<()>,
    pub input: SimpleInput<f64>,
}

impl From<NoiseInput> for ModuleIO {
    fn from(value: NoiseInput) -> Self {
        todo!()
    }
}

impl ModuleBuilder for NoiseInput {
    type Config = NoiseInputConfig;
    type Error = InputConfigError;

    async fn try_build(cfg: &NoiseInputConfig) -> Result<NoiseInput, InputConfigError>  {
        if cfg.max < cfg.min {
            Err(
                InputConfigError::from_str("Must have max >= min for Noise input.")
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
                        warn!("error sending value from noise. sutting down! {:?}", err);
                        break;
                    }

                    sleep(Duration::from_millis(period_ms)).await;
                }
            }); 

            Ok(
                NoiseInput { handle, input }
            )
        }
    }

}

// impl ModuleBuilder for NoiseInput {
//     // type Config = NoiseInputConfig;
//     // type Error = InputConfigError;


//     fn input(&self, name: &str) -> Option<InputKind> {
//         if name.eq("value") { 
//             Some(InputKind::Float(&self.input))
//         } else {
//             None
//         }
//     }

//     fn output(&self, _name: &str) -> Option<OutputKind> {
//         None
//     }
    
//     fn inputs<'a>(&'a self) -> HashMap<String, InputKind<'a>> {
//         HashMap::from([
//             ("value".to_string(), InputKind::Float(&self.input))
//         ])
//     }
    
//     fn join_handle(self) -> tokio::task::JoinHandle<()> {
//         self.handle
//     }
// }
