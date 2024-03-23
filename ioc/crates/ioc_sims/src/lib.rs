extern crate peroxide;
use std::{sync::{Arc, Mutex}, time::Instant};

use ioc_core::Input;
use tracing::info;

pub mod damped_oscillator;

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

pub struct InputAverager {
    state: Arc<Mutex<WindowAverageState>>,
}

impl InputAverager {
    pub fn new(i: &Input<f64>) -> Self {
        let mut rx = i.source();
        let state = Arc::new(Mutex::new(
            WindowAverageState::new(*rx.borrow_and_update())
        ));

        let task_state = state.clone();
        tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let new_value = *rx.borrow_and_update();
                let mut state = match task_state.lock() {
                    Ok(v) => v,
                    Err(poisoned) => poisoned.into_inner(),
                };
                state.append(new_value);
            }
            info!("input average shut down!");
        });

        Self { state }
    }

    pub fn read(&mut self) -> f64 {
        let mut state = match self.state.lock() {
            Ok(v) => v,
            Err(poisoned) => poisoned.into_inner(),
        };
        state.step()
    }
}