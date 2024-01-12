
use std::time::Instant;

use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::error;

use crate::{InputSource,OutputSink, config::{ControllerBuilder, BoxedPorts, ControllerBuilderError}};

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
    fn step(&mut self) -> f64 {
        let now = Instant::now();
        let new_err = self.set_point - self.process_var;
        let dt = now.duration_since(self.last_update).as_secs_f64();

        let drv = if dt > 0.0 {
            (new_err - self.last_err) / dt
        } else {
            0.0
        };

        let int = self.integral_sum + dt * new_err;

        self.last_update = now;
        self.last_err = new_err;
        self.integral_sum = int;

        let o = self.p * new_err + self.i * int + self.d * drv;

        o.max(-1.0).min(1.0)
    }
}

pub struct PidController{
    pub handle: JoinHandle<()>,
}

impl PidController {
    pub fn new(
        p: InputSource<f64>,
        i: InputSource<f64>,
        d: InputSource<f64>,
        set_point: InputSource<f64>,
        process_var: InputSource<f64>,
        output: OutputSink<f64>,
    ) -> Self {

        let handle = tokio::spawn(async move {

            let mut pid = PidState {
                p: p.start,
                i: i.start,
                d: d.start,
                set_point: set_point.start,
                process_var: process_var.start,
                last_update: Instant::now(),
                last_err: set_point.start - process_var.start,
                integral_sum: 0.0,
            };

            let out = pid.step();

            output.tx.send(out).await.unwrap();

            let mut p_rx = p.rx;
            let mut i_rx = i.rx;
            let mut d_rx = d.rx;
            let mut sp_rx = set_point.rx;
            let mut pv_rx = process_var.rx;

            loop {
                tokio::select! {
                    p_res = p_rx.recv() => {
                        match p_res {
                            Ok(new_p) => {
                                pid.p = new_p;
                                if let Err(err) = output.tx.send(pid.step()).await {
                                    error!("pid send error p: {:?}", err);
                                }
                            }
                            Err(err) => {
                                error!("p receive error! {:?}", err);
                            }
                        }
                    }

                    i_res = i_rx.recv() => {
                        match i_res {
                            Ok(new_i) => {
                                pid.i = new_i;
                                if let Err(err) = output.tx.send(pid.step()).await {
                                    error!("pid send error i: {:?}", err);
                                }
                            }
                            Err(err) => {
                                error!("i receive error! {:?}", err);
                            }
                        }
                    }

                    d_res = d_rx.recv() => {
                        match d_res {
                            Ok(new_d) => {
                                pid.d = new_d;
                                if let Err(err) = output.tx.send(pid.step()).await {
                                    error!("pid send error d: {:?}", err);
                                }
                            }
                            Err(err) => {
                                error!("p receive error! {:?}", err);
                            }
                        }
                    }

                    sp_res = sp_rx.recv() => {
                        match sp_res {
                            Ok(new_sp) => {
                                if pid.set_point != new_sp {
                                    pid.set_point = new_sp;
                                }
                                if let Err(err) = output.tx.send(pid.step()).await {
                                    error!("pid send error sp: {:?}", err)
                                }
                            },
                            Err(err) => {
                                error!("setpoint receive error! {:?}", err);
                            },
                        }
                        
                    }
                   
                    pv_res = pv_rx.recv() => {
                        match pv_res {
                            Ok(new_pv) => {
                                if pid.process_var != new_pv {
                                    pid.process_var = new_pv;
                                }
                                if let Err(err) = output.tx.send(pid.step()).await {
                                    error!("pid send error 2 {:?}", err);
                                }
                            },
                            Err(err) => {
                                error!("pv receive err: {:?}", err);
                            }
                        }   
                    }
                }
            }
        });


        PidController { handle: handle }
    }
}


#[derive(Deserialize, Debug)]
pub struct PidControllerConfig {
    p: String,
    i: String,
    d: String,
    set_point: String,
    proc_var: String,
    output: String,
}

impl ControllerBuilder for PidControllerConfig {
    fn try_build(&self, ports: &BoxedPorts) -> Result<JoinHandle<()>, ControllerBuilderError> {

        match (
            ports.get_float_source(&self.p),
            ports.get_float_source(&self.i),
            ports.get_float_source(&self.d),
            ports.get_float_source(&self.set_point),
            ports.get_float_source(&self.proc_var),
            ports.get_float_sink(&self.output)
        ) {
            (Ok(p), Ok(i), Ok(d), Ok(sp), Ok(pv), Ok(out)) => {
                let controller = PidController::new(
                    p, 
                    i, 
                    d, 
                    sp, 
                    pv, 
                    out, 
                );

                Ok(controller.handle)
            },
            (p, i, d, sp, pv, out) => {
                let mut errs: Vec<ControllerBuilderError> = Vec::with_capacity(6);
                for x in vec![p,i,d,sp,pv] {
                    if let Err(e) =x { errs.push(e) }
                }
                if let Err(e) = out { errs.push(e) }
                Err(ControllerBuilderError::from_errors(errs))            
            }
        }
    }
}
