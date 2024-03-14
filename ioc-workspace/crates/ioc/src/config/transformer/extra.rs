use std::collections::{HashMap, HashSet};

use ioc_core::{error::IocBuildError, InputKind, Transformer, TransformerI};
use ioc_extra::transform::{hbridge::{HBridge, HBridgeConfig}, linear::{LinearTransform, LinearTransformConfig}};
use super::TransformerConfig;

use serde::Deserialize;


#[derive(Deserialize,Debug)]
pub struct HBridgeTransformerConfig {
    input: String
}

impl TransformerConfig for HBridgeTransformerConfig {
    async fn try_build(&self,  upstream_inputs: &HashMap<String, InputKind>) -> Result<TransformerI, IocBuildError> {
        let input = match upstream_inputs.get(&self.input) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => return Err(IocBuildError::from_string(format!("unable to build hbridge from non-float input of type {x:?}"))),
            None => return Err(IocBuildError::from_string(format!("no input with name {}",self.input))),
        };
        let cfg = HBridgeConfig{
            input
        };
        let hbridge = HBridge::try_build(&cfg).await?;

        Ok(TransformerI{
            join_handle: hbridge.join_handle,
            inputs: HashMap::from([
                ("forward".to_owned(), InputKind::float(hbridge.forward)),
                ("reverse".to_owned(), InputKind::float(hbridge.reverse)),
                ("enable".to_owned(), InputKind::float(hbridge.enable)),
            ])
        })
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        HashSet::from([&self.input])
    }
}
#[derive(Deserialize,Debug)]

pub struct LinearTransformerConfig {
    input: String,
    from: Vec<f64>,
    to: Vec<f64>,
}

impl TransformerConfig for LinearTransformerConfig {
    async fn try_build(&self,  upstream_inputs: &HashMap<String, InputKind>) -> Result<TransformerI, IocBuildError> {
        if self.from.len() != 2 || self.to.len() != 2 {
            return Err(
                IocBuildError::message("LinearTransform must have exactly two values for the 'from' and 'to' fields")
            )
        }
        let input = match upstream_inputs.get(&self.input) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => return Err(IocBuildError::from_string(format!("unable to build linear transformer from non-float input of type {x:?}"))),
            None => return Err(IocBuildError::from_string(format!("no input with name {}",self.input))),
        };
        let lcfg = LinearTransformConfig::from_ranges(
            input, 
            &[self.from[0], self.from[1]], 
            &[self.to[0], self.to[1]]
        )?;
        let xform = LinearTransform::try_build(&lcfg).await?;
        Ok(
            TransformerI{
                join_handle: xform.join_handle,
                inputs: HashMap::from([
                    ("value".to_string(), InputKind::float(xform.value))
                ])
            }
        )
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        HashSet::from([&self.input])
    }
}
