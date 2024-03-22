extern crate peroxide;
use std::{sync::{Arc, Mutex}, time::Instant};

use ioc_core::{Input, InputSource, Output, OutputSink};
use peroxide::fuga::*;
use tokio::sync::{broadcast, mpsc};
use tracing::info;

pub mod damped_oscillator;


//TODO merge this with SimpleOutput from ioc_extra
pub struct SimOut<T> {
    pub tx: mpsc::Sender<T>,
}

impl<T> Output<T> for SimOut<T> {
    fn sink(&self) -> OutputSink<T> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}

//TODO merge this with SimpleInput from ioc_extra
pub struct SimIn<T> {
    start: T,
    rx: broadcast::Receiver<T>,
}

impl <T: Clone> Input<T> for SimIn<T> {
    fn source(&self) -> InputSource<T> {
        InputSource {
            start: self.start.clone(),
            rx: self.rx.resubscribe(),
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

pub struct InputAverager {
    state: Arc<Mutex<WindowAverageState>>,
}

impl InputAverager {
    pub fn new(i: &dyn Input<f64>) -> Self {
        let src = i.source();
        let state = Arc::new(Mutex::new(
            WindowAverageState::new(src.start)
        ));
        let mut rx = src.rx;

        let task_state = state.clone();
        tokio::spawn(async move {
            while let Ok(new_value) = rx.recv().await {
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


/* 

pub fn main() {

    let mut ex_test = ExplicitODE::new(f);

    let init_state: State<f64> = State::new(
        0.0,
        vec![10.0, 1.0],
        vec![1.0, 0.0],
    );

    ex_test
    .set_initial_condition(init_state)
    .set_method(ExMethod::RK4)
    .set_step_size(0.01f64)
    .set_times(10000);

// =========================================
//  Save results
// =========================================
let results = ex_test.integrate();

// Extract data
let mut df = DataFrame::new(vec![]);
df.push("x_rx4", Series::new(results.col(1)));
df.push("dx_rk4", Series::new(results.col(2)));

    // Write netcdf file (`nc` feature required)
    df.write_csv("spring.csv")
    .expect("Can't write lorenz.nc");
}

fn f(st: &mut State<f64>, _: &NoEnv) {
    let x = &st.value;
    let dx = &mut st.deriv;
    dx[0] = x[1];
    dx[1] = (-k/m)*x[0] - (c/m)*x[1];
}

const m:f64 = 2.0;
const k:f64 = 1.0;
const c:f64 = 0.5;

fn g(st: &mut State<AD>, _: &NoEnv) {
    let x = &st.value;
    let dx = &mut st.deriv;
    dx[0] = x[1];
    dx[1] = (k/m)*x[0] + (c/m)*x[1];
}


*/