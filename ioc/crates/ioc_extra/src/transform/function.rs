
use tokio::task::JoinHandle;
use tokio::sync::broadcast;
use crate::input::SimpleInput;
use ioc_core::Input;
use tracing::{error,debug};
use std::marker::Send;

pub struct FunctionTransformer<O: Clone + Send + 'static> {
    pub join_handle: JoinHandle<()>,
    pub value: SimpleInput<O>,
}

impl <O: Clone + Send + 'static> FunctionTransformer<O> {
    pub fn new<I: Clone + Send + 'static, F: Fn(I) -> O + Send + 'static>(
        input: &dyn Input<I>,
        function: F,
    ) -> Self {
        let (tx, rx) = broadcast::channel(10);
        let source = input.source();
        let start = function(source.start);
        let value = SimpleInput::new(start, rx);
        let mut in_rx = source.rx;
        let join_handle = tokio::spawn(async move {
            while let Ok(x) = in_rx.recv().await {
                if let Err(err) = tx.send(function(x)) {
                    error!("send error in function transformer {}", err);
                    break;
                }
            }
            debug!("shutting down function transformer!");
        });
        Self{
            join_handle,
            value
        }
    }
}