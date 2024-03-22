
use ioc_core::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use tokio::{sync::broadcast, task::JoinHandle};
use tracing::warn;
use std::{collections::HashMap, time::{Duration, Instant}};

use peroxide::{fuga::{ExMethod, NoEnv, ODE}, numerical::ode::{ExplicitODE, State}};

use crate::{InputAverager, SimIn};

pub struct DampedOscillator {
    pub join_handle: JoinHandle<()>,
    pub x: SimIn<f64>, //position
    pub v: SimIn<f64>, //velocity
}

impl From<DampedOscillator> for TransformerI {
    fn from(oscillator: DampedOscillator) -> Self {
        Self{
            join_handle: oscillator.join_handle,
            inputs: HashMap::from([
                ("x".to_string(), InputKind::float(oscillator.x)),
                ("v".to_string(), InputKind::float(oscillator.v)),
            ]),
        }
    }
}



fn spawn_sim_task(cfg: &DampedOscillatorConfig, x_tx: broadcast::Sender<f64>, v_tx: broadcast::Sender<f64>) -> JoinHandle<()> {
    let mut m = InputAverager::new(cfg.m);
    let mut c = InputAverager::new(cfg.c);
    let mut k = InputAverager::new(cfg.k);
    let mut f = InputAverager::new(cfg.f);

    let period_ms = cfg.period_ms;
    let steps_per_frame = cfg.steps_per_frame;

    tokio::spawn(async move {
        //todo: make initial conditions configurable
        let mut x = 0.0;
        let mut v = 0.0;

        loop {
            let m = m.read();
            let c = c.read();
            let k = k.read();
            let f = f.read();

            let start = Instant::now();

            let mut ode = ExplicitODE::new(oscillator_fn);

            let state = State::new(
                0.0,
                vec![x, v, m, c, k, f],
                vec![v, 0.0, 0.0, 0.0, 0.0, 0.0],
            );

            let step_size = period_ms as f64 / 1000.0 / steps_per_frame as f64;

            let frame_sim = ode
                .set_initial_condition(state)
                .set_method(ExMethod::RK4)
                .set_step_size(step_size)
                .set_times(steps_per_frame as usize)
                .integrate();

            let last_row = frame_sim.row(frame_sim.row - 1);
            x = last_row[1];
            v = last_row[2];

            x_tx.send(x).expect("failed to send x from damped oscillator sim");
            v_tx.send(v).expect("failed to send v from damped oscillator sim");

            let end = Instant::now();
            let dt = end - start;

            if dt.as_millis() < period_ms as u128 {
                tokio::time::sleep(Duration::from_millis(period_ms - dt.as_millis() as u64)).await; 
            } else {
                warn!("sim task took too long to run: {}ms", dt.as_millis());
            }
        }
    })
}

fn oscillator_fn(state: &mut State<f64>, _: &NoEnv) {
    let x = &state.value;
    let dx = &mut state.deriv;

    let m = x[2];
    let c = x[3];
    let k = x[4];
    let f = x[5];

    dx[0] = x[1];
    dx[1] =  (f - c * x[1] - k * x[0]) / m;
}


pub struct DampedOscillatorConfig<'a> {
    pub m: &'a dyn Input<f64>, //mass - must be greater than zero
    pub c: &'a dyn Input<f64>, //damping coefficient - must be greater than or equal to zero
    pub k: &'a dyn Input<f64>, //spring constant - must be greater than zero
    pub f: &'a dyn Input<f64>, //external force
    pub period_ms: u64, //how frequently to emit a frame
    pub steps_per_frame: u64, //how many integration steps to take per frame
}

impl<'a> Transformer<'a> for DampedOscillator {
    type Config = DampedOscillatorConfig<'a>;

    async fn try_build(cfg: &Self::Config) -> Result<Self, IocBuildError> {
        
        let (x_tx, x_rx) = broadcast::channel(10);
        let x = SimIn{
            start: 0.0,
            rx: x_rx,
        };

        let (v_tx, v_rx) = broadcast::channel(10);
        let v = SimIn{
            start: 0.0,
            rx: v_rx,
        };

        let join_handle = spawn_sim_task(cfg, x_tx, v_tx);

        Ok(DampedOscillator { join_handle, x, v })
    }
}