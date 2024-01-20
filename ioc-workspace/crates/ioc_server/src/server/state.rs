use crate::error::ServerBuildError;
use crate::{ServerInputConfig, ServerOutputConfig};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(crate) struct ServerState {
    pub handle: JoinHandle<()>,
    pub cmd_tx: mpsc::Sender<StateCmd>,
}

pub(crate) enum StateCmd {
    Foo(String),
}

impl ServerState {
    pub(crate) fn try_build(
        inputs: &HashMap<&str, ServerInputConfig>,
        outputs: &HashMap<&str, ServerOutputConfig>,
    ) -> Result<Self, ServerBuildError> {
        todo!();
    }
}
