use std::collections::HashMap;

use ioc_core::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use tokio::{sync::broadcast, task::JoinHandle, time::sleep};
use tracing::{info, warn};
use std::time::{Instant,Duration};
use std::sync::{Arc, Mutex};
use crate::input::SimpleInput;

pub struct WindowAverageFilterConfig<'a> {
    pub input: &'a dyn Input<f64>,
    pub period_ms: u64,
}

pub struct WindowAverage {
    pub join_handle: JoinHandle<()>,
    pub value: SimpleInput<f64>,
}

impl From<WindowAverage> for TransformerI {
    fn from(avg: WindowAverage) -> Self {
        TransformerI{
            join_handle: avg.join_handle,
            inputs: HashMap::from([
                ("value".to_owned(), InputKind::float(avg.value))
            ])
        }
    }
}

struct WindowAverageState{
    last_value: f64,
    last_append: Instant,
    last_window: Instant,
    sum: f64
}

impl WindowAverageState {
    fn new(start: f64) -> Self {
        Self {
            last_value: start, 
            last_append: Instant::now(),
            last_window: Instant::now(),
            sum: 0.0,
        }
    }
    fn append(&mut self, new_value: f64) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_append).as_secs_f64();
        self.sum += dt * self.last_value;

        self.last_value = new_value;
        self.last_append = now;
    }
    fn step(&mut self) -> f64 {
        let now = Instant::now();
        if self.last_window == self.last_append {
            //only one remainn-sum, so just send the last value 
            self.last_window = now;
            self.last_append = now;
            self.sum = 0.0;
            self.last_value
        } else {
            //add a reaimann-sum and divide by dt for average. reset.
            let dt = now.duration_since(self.last_append).as_secs_f64();
            let window_t = now.duration_since(self.last_window).as_secs_f64();
            self.sum += dt * self.last_value;
            let avg = self.sum / window_t;
            self.last_window = now;
            self.last_append = now;
            self.sum = 0.0;
            avg
        }
    }
}

fn spawn_window_avg_task(
    start: f64,
    mut in_rx: broadcast::Receiver<f64>,
    out_tx: broadcast::Sender<f64>,
    period_ms: u64,
) -> JoinHandle<()> {


    let state = Arc::new(Mutex::new(WindowAverageState::new(start)));

    let wt_state = state.clone();
    let write_task = tokio::spawn(async move {
        loop{
            let step = match wt_state.lock() {
                Ok(mut state) => state.step(),
                Err(poisoned) => poisoned.into_inner().step(),
            };
            if let Err(err) = out_tx.send(step) {
                warn!("send error in window averager: {}", err);
                break;
            }
            sleep(Duration::from_millis(period_ms)).await;
        }
        info!("write task done in window averager!");
    });

    //read task 
    tokio::spawn(async move {
        while let Ok(new_in) = in_rx.recv().await {
            let mut state = match state.lock() {
                Ok(state) => state,
                Err(poisoned) => poisoned.into_inner(),
            };
            state.append(new_in);
        }
        info!("shutting down window averager!");
        write_task.abort();
    })
}

impl <'a> Transformer<'a> for WindowAverage {
    type Config = WindowAverageFilterConfig<'a>;

    async fn try_build(cfg: &WindowAverageFilterConfig<'a>) -> Result<WindowAverage, IocBuildError> {
        let in_src = cfg.input.source();
        let in_rx = in_src.rx;
        let start = in_src.start;

        let (out_tx, out_rx) = broadcast::channel(10);
        let value = SimpleInput::new(start, out_rx);

        let join_handle = spawn_window_avg_task(start, in_rx, out_tx, cfg.period_ms);
        Ok(WindowAverage{
            join_handle,
            value
        })
    }
}