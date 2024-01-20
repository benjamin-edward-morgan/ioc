pub(crate) mod state;

use crate::error::ServerBuildError;
use crate::{ServerInputConfig, ServerOutputConfig, TypedInput, TypedOutput};

use ioc_core::{Input, InputSource, Output, OutputSink};
use state::StateCmd;
use std::marker::PhantomData;
use tokio::sync::mpsc;

pub struct ServerInput<T> {
    p: PhantomData<T>,
}

impl<T> Input<T> for ServerInput<T> {
    fn source(&self) -> InputSource<T> {
        todo!()
    }
}

pub struct ServerInputBuilder;

impl ServerInputBuilder {
    pub(crate) fn try_build(
        cmd_tx: &mpsc::Sender<StateCmd>,
        config: ServerInputConfig,
    ) -> Result<TypedInput, ServerBuildError> {
        todo!()
    }
}

pub struct ServerOutput<T> {
    p: PhantomData<T>,
}

impl<T> Output<T> for ServerOutput<T> {
    fn sink(&self) -> OutputSink<T> {
        todo!()
    }
}

pub struct ServerOutputBuilder;

impl ServerOutputBuilder {
    pub(crate) fn try_build(
        cmd_tx: &mpsc::Sender<StateCmd>,
        config: ServerOutputConfig,
    ) -> Result<TypedOutput, ServerBuildError> {
        todo!()
    }
}
