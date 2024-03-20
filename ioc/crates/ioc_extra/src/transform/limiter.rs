use std::collections::HashMap;

use ioc_core::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use tokio::{sync::broadcast, task::JoinHandle, time::sleep};
use tracing::{debug,info};
use std::time::{Instant,Duration};
use crate::input::SimpleInput;

#[derive(Clone)]
pub struct LimiterParams {
    pub min: f64,
    pub max: f64,
    pub dmin: f64,
    pub dmax: f64,
    pub ddmin: f64,
    pub ddmax: f64,
    pub period_ms: u64,
}

impl Default for LimiterParams {
    fn default() -> Self {
        Self{
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            dmin: f64::NEG_INFINITY,
            dmax: f64::INFINITY,
            ddmin: f64::NEG_INFINITY,
            ddmax: f64::INFINITY,
            period_ms: 25,
        }
    }
}

pub struct LimiterFilterConfig<'a> {
    pub input: &'a dyn Input<f64>,
    pub params: LimiterParams,
}

pub struct Limiter {
    pub join_handle: JoinHandle<()>,
    pub value: SimpleInput<f64>,
}

impl From<Limiter> for TransformerI {
    fn from(limiter: Limiter) -> Self {
        TransformerI{
            join_handle: limiter.join_handle,
            inputs: HashMap::from([
                ("value".to_owned(), InputKind::float(limiter.value))
            ]),
        }
    }
}

struct State {
    target: f64, //the desired value of x
    x: f64, //current value of x
    dx: f64, //current derivative of x
    ddx: f64, //current acceleration of x
    last_time: Instant, //timestamp of the last step
}

impl State {
    fn initial(start: f64, params: &LimiterParams) -> Result<State, IocBuildError> {
        if params.min > params.max || params.min.is_nan() || params.max.is_nan() {
            return Err(IocBuildError::message("must have min < max and non NaN values in limiter"));
        }
        if params.dmin > params.dmax || params.dmin.is_nan() || params.dmax.is_nan() {
            return Err(IocBuildError::message("must have dmin < dmax and non NaN values in limiter"));
        }
        if params.ddmin > params.ddmax || params.ddmin.is_nan() || params.ddmax.is_nan() {
            return Err(IocBuildError::message("must have ddmin < ddmax and non NaN values in limiter"));
        }
        let x = start.max(params.min).min(params.max);
        Ok(State{target: x, x: x, dx: 0.0, ddx: 0.0, last_time: Instant::now()})
    }

    fn step(&mut self, params: &LimiterParams) -> f64 {
        let now = Instant::now();

        if self.x == self.target && self.dx == 0.0 && self.ddx == 0.0 {
            self.last_time = now;
            self.x
        } else {
            let dt = now.duration_since(self.last_time).as_secs_f64();
            //move with whatever velocity we had last frame
            self.x = (self.x + self.dx * dt).min(params.max).max(params.min);
            self.last_time = now; 

            //can we reach the target with an accel and deccel step (disregarding velocity limit)
            let (accel, deccel) = if self.target > self.x {
                (params.ddmax, params.ddmin)
            } else {
                (params.ddmin, params.ddmax)
            };
            if let Some((t1, t2)) = calc_accel_decel_times(deccel, accel, self.x, self.dx, self.target) {
                if (t1 + t2) * 1000.0 < (params.period_ms as f64) {
                    //if we can reach the target in less than one frame, 
                    //just go ahead and arrive there 
                    self.x = self.target;
                    self.dx = 0.0;
                    self.ddx = 0.0;
                } else {
                    if t1 < 0.0001 {
                        //skip the accel period 
                        self.ddx = deccel;
                    } else {
                        self.ddx = accel;
                    }
                }
            } else {
                //we are going to overshoot. deccelerate anyway
                self.ddx = deccel;
            }

            //calculate new velocity (for next frame)
            self.dx = (self.dx + self.ddx * dt).max(params.dmin).min(params.dmax);

            //return x
            self.x
        }

    }
}


///suppose starting at position x0 and velocity v0 we want to arrive at position xf in the shortest possible time 
/// with velocity 0, moving without friction by accelerating at constant acceleration for time t1, then decelerating 
/// for time t2. 
/// This function calculates t1 and t2 provided that:
/// if xf > x0, then accel must be positive and deccel must be negative 
/// if xf < x0, then accel must be negative and deccel must be positive
/// we can arrive at xf without overshooting
fn calc_accel_decel_times(deccel: f64, accel: f64, x0: f64, v0: f64, xf: f64) -> Option<(f64, f64)> {
    let aa = accel / 2.0 - accel * accel / deccel / 2.0;
    let bb = v0 - 3.0 * v0 * accel / deccel;
    let cc = x0 - v0 * v0 / deccel / 2.0 - xf;
    let disc = (bb * bb) - (4.0 * aa * cc); //quadratic formula discriminant
    if disc < 0.0 {
        None
    } else {
        let t1 = [(-bb + disc.sqrt()) / 2.0 / aa, (-bb - disc.sqrt()) / 2.0 / aa]
            .into_iter()
            .filter(|i| *i >= 0.0)
            .fold(f64::NAN, f64::max);
        if !t1.is_nan() {
            let t2 = (-v0 - accel * t1) / deccel;
            if t2 >= 0.0 {
                Some((t1, t2))
            } else {
                None
            }
        } else {
            None
        }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn test_calc_accel_decel_times() {
        //x0=0, v0=0, xf=5. accelerate at 1 for sqrt(5) secs, decelerate at -1 for sqrt(5) secs
        let times = super::calc_accel_decel_times(-1.0, 1.0, 0.0, 0.0, 5.0);
        assert_eq!(times, Some(((5.0_f64).sqrt(), (5.0_f64).sqrt())));

        //x0=0, v0=0, xf=-5. accelerate at -1 for sqrt(5) secs, decelerate at 1 for sqrt(5) secs
        let times = super::calc_accel_decel_times(1.0, -1.0, 0.0, 0.0, -5.0);
        assert_eq!(times, Some(((5.0_f64).sqrt(), (5.0_f64).sqrt())));

        //x0=0, v0=1, xf=5. accelerate at 1 for t1=(sqrt(34)-4)/5 secs, decelerate at -1 for t1 + 1 secs
        let times = super::calc_accel_decel_times(-1.0, 1.0, 0.0, 1.0, 5.0);
        let expected_t1 = ((34.0_f64).sqrt()-4.0)/2.0;
        assert_eq!(times, Some((expected_t1, expected_t1+1.0)));

        //overshooting test (same as the first but v0 = 100)
        let times = super::calc_accel_decel_times(-1.0, 1.0, 0.0, 100.0, 5.0);
        assert_eq!(times, None);
    }
}






fn spawn_limiter_task(
    mut state: State, 
    params: &LimiterParams, 
    mut rx: broadcast::Receiver<f64>,
    tx: broadcast::Sender<f64>
) -> JoinHandle<()> {
    let params = params.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = sleep(Duration::from_millis(params.period_ms)) => {},
                in_res = rx.recv() => {
                    if let Ok(new_in) = in_res {
                        state.target = new_in;
                    } else {
                        break;
                    }
                }
            }
            if let Err(_) = tx.send(state.step(&params)) {
                break;
            }
        }
        debug!("limiter task shutting down!");
    })
}

impl<'a> Transformer<'a> for Limiter {
    type Config = LimiterFilterConfig<'a>;

    async fn try_build(cfg: &LimiterFilterConfig<'a>) -> Result<Limiter, IocBuildError> {

        let params = &cfg.params;
        let in_src = cfg.input.source();
        let in_rx = in_src.rx;
        let start = in_src.start;
        let state = State::initial(start, params)?;
        let (out_tx, out_rx) = broadcast::channel(10);
        let value = SimpleInput::new(start, out_rx);

        let join_handle = spawn_limiter_task(state, params, in_rx, out_tx);

        Ok(Limiter{
            join_handle,
            value,
        })
    }
}