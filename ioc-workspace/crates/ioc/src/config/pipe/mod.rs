use std::collections::HashMap;

use ioc_core::{error::IocBuildError, pipe::Pipe, InputKind, OutputKind};
use serde::Deserialize;

#[derive(Deserialize,Debug)]
pub struct PipeConfig{
    pub from: String,
    pub to: String,
}

impl PipeConfig{
    pub fn try_build(
        &self,
        inputs: &HashMap<String, InputKind>,
        outputs: &HashMap<String, OutputKind>,
    ) -> Result<Pipe, IocBuildError> {
        let input = inputs.get(&self.from);
        let output = outputs.get(&self.to);

        match (input, output) {
            (Some(InputKind::String(input)), Some(OutputKind::String(output))) => Ok(Pipe::new(input.as_ref(), output.as_ref())),
            (Some(InputKind::Binary(input)), Some(OutputKind::Binary(output))) => Ok(Pipe::new(input.as_ref(), output.as_ref())),
            (Some(InputKind::Float(input)), Some(OutputKind::Float(output))) => Ok(Pipe::new(input.as_ref(), output.as_ref())),
            (Some(InputKind::Bool(input)), Some(OutputKind::Bool(output))) => Ok(Pipe::new(input.as_ref(), output.as_ref())),
            (Some(InputKind::Array(input)), Some(OutputKind::Array(output))) => Ok(Pipe::new(input.as_ref(), output.as_ref())),
            (Some(input), Some(output)) => {
                Err(
                    IocBuildError::from_string(
                    format!("got mismatched types when trying to build Pipe from {} to {}. types were {:?} and {:?} respectively", self.from, self.to, input, output))
                )
            },
            (input, output) => {
                let mut errs = Vec::with_capacity(2);
                if input.is_none() {
                    errs.push(format!("can't build Pipe from {} to {}. input {} not found.", self.from, self.to, self.from));
                }
                if output.is_none() {
                    errs.push(format!("can't build Pipe from {} to {}. output {} not found.", self.from, self.to, self.to));
                }
                Err(IocBuildError::messages(&errs))
            }   
        }
    }
}