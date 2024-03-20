use std::collections::{HashMap, HashSet};

use super::TransformerConfig;
use ioc_core::{
    error::IocBuildError,
    transformer::{Sum, SumConfig},
    InputKind, Transformer, TransformerI,
};

use serde::Deserialize;

///Creates a transformer that reads Float values from any number of inputs, emits an input named 'value' which is their sum. 
#[derive(Deserialize, Debug)]
pub struct SumTransformerConfig {
    pub inputs: Vec<String>,
}

impl TransformerConfig for SumTransformerConfig {
    async fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> Result<TransformerI, IocBuildError> {
        let mut inputs = Vec::with_capacity(self.inputs.len());
        let mut errors = Vec::with_capacity(inputs.len());
        self.inputs
            .iter()
            .for_each(|input_key| match upstream_inputs.get(input_key) {
                Some(InputKind::Float(float)) => inputs.push(float.as_ref()),
                Some(other) => errors.push(IocBuildError::from_string(format!(
                    "Expected input '{input_key}' to be a Float but got {other:?}"
                ))),
                None => errors.push(IocBuildError::from_string(format!(
                    "No input named '{input_key}'"
                ))),
            });

        if !errors.is_empty() {
            return Err(IocBuildError::from_errs(errors));
        }

        let cfg = SumConfig { inputs: inputs };
        let sum = Sum::try_build(&cfg).await?;
        Ok(TransformerI {
            join_handle: tokio::spawn(async move { println!("this is wrong! ") }), //todo
            inputs: HashMap::from([("value".to_owned(), InputKind::float(sum.value))]),
        })
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        self.inputs.iter().collect()
    }
}
