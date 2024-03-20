use ioc_core::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use tokio::{sync::broadcast, time::sleep, task::JoinHandle};
use std::collections::HashMap;
use std::time::{Instant,Duration};
use tracing::debug;
use crate::input::SimpleInput;

pub struct PidConfig<'a> {
    pub set_point: &'a dyn Input<f64>,
    pub process_var: &'a dyn Input<f64>,
    pub p: &'a dyn Input<f64>,
    pub i: &'a dyn Input<f64>,
    pub d: &'a dyn Input<f64>,
    pub period_ms: u16,
}

pub struct Pid {
    pub join_handle: JoinHandle<()>,
    pub value: SimpleInput<f64>,
}

impl From<Pid> for TransformerI {
    fn from(pid: Pid) -> Self {
        TransformerI{
            join_handle: pid.join_handle,
            inputs: HashMap::from([
                ("value".to_string(), InputKind::float(pid.value)),
            ]),
        }
    }
}

impl<'a> Transformer<'a> for Pid {
    type Config = PidConfig<'a>;

    async fn try_build(cfg: &PidConfig<'a>) -> Result<Pid, IocBuildError> {

        let (value_tx, value_rx) = broadcast::channel(10);
        let set_point = cfg.set_point.source();
        let process_var = cfg.process_var.source();
        let p = cfg.p.source();
        let i = cfg.i.source();
        let d = cfg.d.source();

        let mut state = PidState::new(
            p.start,
            i.start,
            d.start,
            set_point.start,
            process_var.start,
        );
        //start with just a p component for the output
        let start_value = state.last_err * p.start;
        let value = SimpleInput::new(start_value, value_rx);

        let mut p_rx = p.rx;
        let mut i_rx = i.rx;
        let mut d_rx = d.rx;
        let mut sp_rx = set_point.rx;
        let mut pv_rx = process_var.rx;
        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(10)) => {},
                    p_res = p_rx.recv() => {
                        if let Ok(p) = p_res {
                            state.p=p;
                        } else {
                            break;
                        }
                    },
                    i_res = i_rx.recv() => {
                        if let Ok(i) = i_res {
                            state.i=i;
                        } else {
                            break;
                        }
                    },
                    d_res = d_rx.recv() => {
                        if let Ok(d) = d_res {
                            state.d=d;
                        } else {
                            break;
                        }
                    },
                    sp_res = sp_rx.recv() => {
                        if let Ok(sp) = sp_res {
                            state.set_point=sp;
                        } else {
                            break;
                        }
                    },
                    pv_res = pv_rx.recv() => {
                        if let Ok(pv) = pv_res {
                            state.process_var=pv;
                        } else {
                            break;
                        }
                    },
                };

                let value = state.step();

                if let Err(_) = value_tx.send(value) {
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