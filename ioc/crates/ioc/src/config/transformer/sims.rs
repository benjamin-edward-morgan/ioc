use std::collections::{HashMap, HashSet};

use ioc_sims::damped_oscillator::{DampedOscillatorConfig, DampedOscillator};
use ioc_core::{error::IocBuildError, InputKind, Transformer, TransformerI};
use serde::Deserialize;

use super::TransformerConfig;




#[derive(Debug, Deserialize)]
pub struct DampedOscillatorSimConfig{
    m: String,
    k: String,
    c: String,
    f: String,
    period_ms: u64,
    steps_per_frame: u64,
}

impl TransformerConfig for DampedOscillatorSimConfig {
    async fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> Result<TransformerI, IocBuildError> {
        let m = match upstream_inputs.get(&self.m) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(other) => {
                return Err(IocBuildError::from_string(format!("expected {} to be a Float but got {:?}", self.m, other)))
            },
            _ => return Err(IocBuildError::from_string(format!("could not find input {}", self.m))),
        };

        let k = match upstream_inputs.get(&self.k) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(other) => {
                return Err(IocBuildError::from_string(format!("expected {} to be a Float but got {:?}", self.k, other)))
            },
            _ => return Err(IocBuildError::from_string(format!("could not find input {}", self.k))),
        };

        let c = match upstream_inputs.get(&self.c) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(other) => {
                return Err(IocBuildError::from_string(format!("expected {} to be a Float but got {:?}", self.c, other)))
            },
            _ => return Err(IocBuildError::from_string(format!("could not find input {}", self.c))),
        };

        let f = match upstream_inputs.get(&self.f) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(other) => {
                return Err(IocBuildError::from_string(format!("expected {} to be a Float but got {:?}", self.f, other)))
            },
            _ => return Err(IocBuildError::from_string(format!("could not find input {}", self.f))),
        };

        let config = DampedOscillatorConfig {
            m, k, c, f, period_ms: self.period_ms, steps_per_frame: self.steps_per_frame
        };

        let oscillator = DampedOscillator::try_build(&config).await?;

        Ok(TransformerI{
            join_handle: oscillator.join_handle,
            inputs: HashMap::from([
                ("x".to_string(), InputKind::float(oscillator.x)),
                ("v".to_string(), InputKind::float(oscillator.v)),
            ]),
        })
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        HashSet::from([&self.m, &self.k, &self.c, &self.f])
    }
}