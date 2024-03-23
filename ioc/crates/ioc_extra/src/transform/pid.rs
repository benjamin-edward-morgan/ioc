use ioc_core::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use tokio::{time::sleep, task::JoinHandle};
use std::collections::HashMap;
use std::time::{Instant,Duration};
use tracing::debug;

pub struct PidConfig<'a> {
    pub set_point: &'a Input<f64>,
    pub process_var: &'a Input<f64>,
    pub p: &'a Input<f64>,
    pub i: &'a Input<f64>,
    pub d: &'a Input<f64>,
    pub period_ms: u64,
}

pub struct Pid {
    pub join_handle: JoinHandle<()>,
    pub value: Input<f64>,
}

impl From<Pid> for TransformerI {
    fn from(pid: Pid) -> Self {
        TransformerI{
            join_handle: pid.join_handle,
            inputs: HashMap::from([
                ("value".to_string(), InputKind::Float(pid.value)),
            ]),
        }
    }
}

impl<'a> Transformer<'a> for Pid {
    type Config = PidConfig<'a>;

    async fn try_build(cfg: &PidConfig<'a>) -> Result<Pid, IocBuildError> {
        let mut set_point = cfg.set_point.source();
        let mut process_var = cfg.process_var.source();
        let mut p = cfg.p.source();
        let mut i = cfg.i.source();
        let mut d = cfg.d.source();

        let mut state = PidState::new(
            *p.borrow_and_update(),
            *i.borrow_and_update(),
            *d.borrow_and_update(),
            *set_point.borrow_and_update(),
            *process_var.borrow_and_update(),
        );
        //start with just a p component for the output
        let start_value = state.last_err * *p.borrow_and_update();
        let (value, value_tx) = Input::new(start_value);
        let period_ms = cfg.period_ms;
        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(period_ms)) => {},
                    p_res = p.changed() => {
                        if p_res.is_ok() {
                            state.p= *p.borrow_and_update();
                        } else {
                            break;
                        }
                    },
                    i_res = i.changed() => {
                        if i_res.is_ok() {
                            state.i= *i.borrow_and_update();
                        } else {
                            break;
                        }
                    },
                    d_res = d.changed() => {
                        if d_res.is_ok() {
                            state.d= *d.borrow_and_update();
                        } else {
                            break;
                        }
                    },
                    set_point_res = set_point.changed() => {
                        if set_point_res.is_ok() {
                            state.set_point= *set_point.borrow_and_update();
                        } else {
                            break;
                        }
                    },
                    process_var_res = process_var.changed() => {
                        if process_var_res.is_ok() {
                            state.process_var= *process_var.borrow_and_update();
                        } else {
                            break;
                        }
                    },
                };

                let value = state.step();

                if value_tx.send(value).is_err() {
                    break;
                }
            }
            debug!("pid controller shut down")
        });

        Ok(Pid{
            join_handle,
            value,
        })
    }
}

///internal state of pid controller
struct PidState {
    p: f64,
    i: f64,
    d: f64,
    set_point: f64,
    process_var: f64,
    last_update: Instant,
    last_err: f64,
    integral_sum: f64,
}

impl PidState {
    fn new(p: f64, i: f64, d: f64, set_point: f64, process_var: f64) -> Self {
        PidState {
            p, i, d, set_point, process_var,
            last_update: Instant::now(),
            last_err: set_point-process_var,
            integral_sum: 0.0 
        }
    }
}

impl PidState {
    fn step(&mut self) -> f64 {
        //calculate new error and how long since last update
        let now = Instant::now();
        let new_err = self.set_point - self.process_var;
        let dt = now.duration_since(self.last_update).as_secs_f64();

        //calculate simple numerical derivative
        let drv = if dt > 0.0 {
            (new_err - self.last_err) / dt
        } else {
            0.0
        };

        //add a new area to the reimann sum to calculate numerical integral
        let int = if new_err.is_finite() {
            self.integral_sum + dt * new_err
        } else {
            self.integral_sum
        };

        //update ourself
        self.last_update = now;
        self.last_err = new_err;
        self.integral_sum = int;

        //calculate pid output: p * err + i * integral(err, dt) + d + derivative(err, t)
        self.p * new_err + self.i * int + self.d * drv
    }
}