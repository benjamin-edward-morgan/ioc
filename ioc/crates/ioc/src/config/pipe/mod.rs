use std::collections::HashMap;

use ioc_core::{error::IocBuildError, pipe::Pipe, InputKind, OutputKind};
use serde::Deserialize;

//A `Pipe`` is a simple object that reads values from an `Input` and writes values received to an `Output`
#[derive(Deserialize, Debug)]
pub struct PipeConfig {
    pub from: String,
    pub to: String,
}

/// Implements the `try_build` method for the `PipeConfig` struct.
/// This method attempts to build a `Pipe` object based on the provided inputs and outputs.
/// It returns a `Result` with the built `Pipe` on success, or an `IocBuildError` on failure.
impl PipeConfig {
    pub fn try_build(
        &self,
        inputs: &HashMap<String, InputKind>,
        outputs: &HashMap<String, OutputKind>,
    ) -> Result<Pipe, IocBuildError> {
        let input = inputs.get(&self.from);
        let output = outputs.get(&self.to);

        match (input, output) {
            // If both input and output are of type String, create a new Pipe with the input and output as references.
            (Some(InputKind::String(input)), Some(OutputKind::String(output))) => Ok(Pipe::new(input, output)),
            // If both input and output are of type Binary, create a new Pipe with the input and output as references.
            (Some(InputKind::Binary(input)), Some(OutputKind::Binary(output))) => Ok(Pipe::new(input, output)),
            // If both input and output are of type Float, create a new Pipe with the input and output as references.
            (Some(InputKind::Float(input)), Some(OutputKind::Float(output))) => Ok(Pipe::new(input, output)),
            // If both input and output are of type Bool, create a new Pipe with the input and output as references.
            (Some(InputKind::Bool(input)), Some(OutputKind::Bool(output))) => Ok(Pipe::new(input, output)),
            // If both input and output are of type Array, create a new Pipe with the input and output as references.
            (Some(InputKind::Array(input)), Some(OutputKind::Array(output))) => Ok(Pipe::new(input, output)),
            // If the input and output types do not match, return an error with the mismatched types.
            (Some(input), Some(output)) => {
                Err(
                    IocBuildError::from_string(
                    format!("got mismatched types when trying to build Pipe from {} to {}. types were {:?} and {:?} respectively", self.from, self.to, input, output))
                )
            },
            // If either the input or output is missing, return an error with the missing input/output message.
            (input, output) => {
                let mut errs = Vec::with_capacity(2);
                if input.is_none() {
                    errs.push(format!("can't build Pipe from {} to {}. input {} not found.", self.from, self.to, self.from));
                }
                if output.is_none() {
                    errs.push(format!("can't build Pipe from {} to {}. output {} not found.", self.from, self.to, self.to));
                }
                Err(IocBuildError::messages(&errs))
            },
        }
    }
}
