
use tokio::task::JoinHandle;
use ioc_core::Input;
use tracing::{error,debug};
use std::marker::Send;

pub struct FunctionTransformer<O: Clone + Send + 'static> {
    pub join_handle: JoinHandle<()>,
    pub value: Input<O>,
}

impl <O: Clone + Sync + Send + 'static> FunctionTransformer<O> {
    pub fn new<I: Clone + Sync + Send + 'static, F: Fn(I) -> O + Sync + Send + 'static>(
        input: &Input<I>,
        function: F,
    ) -> Self {
        let mut in_rx = input.source();
        let start = function(in_rx.borrow_and_update().clone());
        let (value, out_tx) = Input::new(start);
        let join_handle = tokio::spawn(async move {
            while in_rx.changed().await.is_ok() {
                let new_in = in_rx.borrow_and_update().clone();
                if let Err(err) = out_tx.send(function(new_in)) {
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