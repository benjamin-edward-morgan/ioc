use crate::{
    config::{BoxedPorts, ControllerBuilder, ControllerBuilderError},
    InputSource, OutputSink,
};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::warn;

pub mod pid;

//directly writes the input to the output
pub struct DirectController {
    pub handle: JoinHandle<()>,
}

impl DirectController {
    pub fn new<T: Send + Clone + 'static>(
        mut source: InputSource<T>,
        sink: OutputSink<T>,
    ) -> DirectController {
        let handle = tokio::spawn(async move {
            if sink.tx.send(source.start).await.is_ok() {
                loop {
                    match source.rx.recv().await {
                        Ok(t) => {
                            if sink.tx.send(t).await.is_err() {
                                break;
                            }
                        }
                        Err(err) => {
                            warn!("direct err: {:?}", err);
                        }
                    }
                }
            }
            warn!("direct controller shutting down!")
        });

        DirectController { handle }
    }
}

#[derive(Deserialize, Debug)]
pub struct DirectControllerConfig {
    input: String,
    output: String,
}

impl DirectControllerConfig {
    pub fn try_build_float(
        &self,
        ports: &BoxedPorts,
    ) -> Result<JoinHandle<()>, ControllerBuilderError> {
        match (
            ports.get_float_source(&self.input),
            ports.get_float_sink(&self.output),
        ) {
            (Ok(input), Ok(output)) => {
                let ctrl = DirectController::new(input, output);
                Ok(ctrl.handle)
            }
            (input, output) => {
                let mut errs = Vec::with_capacity(2);
                if let Err(e) = input {
                    errs.push(e)
                }
                if let Err(e) = output {
                    errs.push(e)
                }
                Err(ControllerBuilderError::from_errors(errs))
            }
        }
    }
    pub fn try_build_bool(
        &self,
        ports: &BoxedPorts,
    ) -> Result<JoinHandle<()>, ControllerBuilderError> {
        match (
            ports.get_bool_source(&self.input),
            ports.get_bool_sink(&self.output),
        ) {
            (Ok(input), Ok(output)) => {
                let ctrl = DirectController::new(input, output);
                Ok(ctrl.handle)
            }
            (input, output) => {
                let mut errs = Vec::with_capacity(2);
                if let Err(e) = input {
                    errs.push(e)
                }
                if let Err(e) = output {
                    errs.push(e)
                }
                Err(ControllerBuilderError::from_errors(errs))
            }
        }
    }
    pub fn try_build_string(
        &self,
        ports: &BoxedPorts,
    ) -> Result<JoinHandle<()>, ControllerBuilderError> {
        match (
            ports.get_string_source(&self.input),
            ports.get_string_sink(&self.output),
        ) {
            (Ok(input), Ok(output)) => {
                let ctrl = DirectController::new(input, output);
                Ok(ctrl.handle)
            }
            (input, output) => {
                let mut errs = Vec::with_capacity(2);
                if let Err(e) = input {
                    errs.push(e)
                }
                if let Err(e) = output {
                    errs.push(e)
                }
                Err(ControllerBuilderError::from_errors(errs))
            }
        }
    }
}

impl ControllerBuilder for DirectControllerConfig {
    fn try_build(&self, ports: &BoxedPorts) -> Result<JoinHandle<()>, ControllerBuilderError> {
        match (
            ports.get_float_source(&self.input),
            ports.get_float_sink(&self.output),
        ) {
            (Ok(input), Ok(output)) => {
                let ctrl = DirectController::new(input, output);
                Ok(ctrl.handle)
            }
            (input, output) => {
                let mut errs = Vec::with_capacity(2);
                if let Err(e) = input {
                    errs.push(e)
                }
                if let Err(e) = output {
                    errs.push(e)
                }
                Err(ControllerBuilderError::from_errors(errs))
            }
        }
    }
}

//transforms the input by the supplied function, sends to output
pub struct TransformController {
    pub handle: JoinHandle<()>,
}

impl TransformController {
    pub fn new<I, O, F>(
        mut source: InputSource<I>,
        sink: OutputSink<O>,
        xform: F,
    ) -> TransformController
    where
        I: Send + Clone + 'static,
        O: Send + Clone + 'static,
        F: Send + Copy + FnOnce(I) -> O + 'static,
    {
        let handle = tokio::spawn(async move {
            if sink.tx.send(xform(source.start)).await.is_ok() {
                while let Ok(i) = source.rx.recv().await {
                    if sink.tx.send(xform(i)).await.is_err() {
                        break;
                    }
                }
                warn!("transform controller shutting down!");
            }
        });

        TransformController { handle }
    }
}
