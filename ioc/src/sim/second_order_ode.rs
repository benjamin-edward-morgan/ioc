use crate::config::{BoxedPorts, ControllerBuilder, ControllerBuilderError};
use crate::sim::ValueAverage;
use crate::{InputSource, OutputSink};

use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::warn;

/*
Simulates the second order inhomogeneous differential equation
a*x'' + b*x' + c = f

handle is the task that recalculates x each period
*/
pub struct SecondOrderOde {
    pub handle: JoinHandle<()>,
}

impl SecondOrderOde {
    /*
    a,b,c and f are variable inputs
    x is the output
    x0, dx0 are the start value, and start derivative
    period_ms is the target number of miliseconds between frames
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        a: InputSource<f64>,
        b: InputSource<f64>,
        c: InputSource<f64>,
        f: InputSource<f64>,
        x: OutputSink<f64>,
        x0: f64,
        dx0: f64,
        period_ms: u16,
    ) -> Self {
        //calculate the averages of the inputs over each window
        //because the inputs could create values much more frequently than period_ms
        let a = ValueAverage::new(a);
        let b = ValueAverage::new(b);
        let c = ValueAverage::new(c);
        let f = ValueAverage::new(f);

        let mut state = SecondOrderState {
            a,
            b,
            c,
            f,
            x: x0,
            dx: dx0,
        };

        let handle = tokio::spawn(async move {
            //write x0 to the output
            x.tx.send(x0).await.unwrap();

            //sleep for the period
            let mut delay = sleep(Duration::from_millis(period_ms.into()));
            let mut last_loop = Instant::now();

            loop {
                delay.await;
                //see how much time actually elapsed
                let dt = last_loop.elapsed().as_secs_f64();
                state.step(dt);
                if let Err(err) = x.tx.send(state.x).await {
                    warn!("second order send error: {:?}", err);
                }

                delay = sleep(Duration::from_millis(period_ms.into()));
                last_loop = Instant::now();
            }
        });

        SecondOrderOde { handle }
    }
}

struct SecondOrderState {
    a: ValueAverage,
    b: ValueAverage,
    c: ValueAverage,
    f: ValueAverage,
    x: f64,
    dx: f64,
}

impl SecondOrderState {
    fn step(&mut self, dt: f64) {
        //get the averages of a,b,c and f over last frame
        let a = self.a.read();
        let b = self.b.read();
        let c = self.c.read();
        let f = self.f.read();

        //a*x'' + b*x' + c*x = f
        // x'' = (f - b*x' - c*x) / a

        // euler's method
        //  x(i+1) = x(i) + dt * x'(i)
        //  x'(i+1) = dt * (f - b*x'(i) - c*x(i)) / a
        // let new_x = self.x + dt * self.dx;
        // let new_dx =self.dx + dt * (f - b*self.dx - c*self.x) / a;

        // midpoint method
        // f([x, x']') = ['x, (f - b*x' - c*x)/a]
        // x(i+1) = x(i) + dt * f( x(i) + dt/2 * f(x(i)) )
        let x_hat = self.x + dt / 2.0 * self.dx;
        let dx_hat = self.dx + dt / 2.0 * (f - b * self.dx - c * self.x) / a;
        let new_x = self.x + dt * dx_hat;
        let new_dx = self.dx + dt * (f - b * dx_hat - c * x_hat) / a;

        // ben morgan's method
        // let new_ddx = (f - c * self.x - b * self.dx) / a;
        // let new_dx = self.dx + new_ddx * dt;
        // let new_x = self.x + new_dx * dt;

        self.x = new_x;
        self.dx = new_dx;
    }
}

#[derive(Deserialize, Debug)]
pub struct SecondOrderOdeConfig {
    a: String, //input names
    b: String,
    c: String,
    f: String,
    x: String,
    x0: f64,
    dx0: f64,
    period_ms: u16,
}

impl ControllerBuilder for SecondOrderOdeConfig {
    fn try_build(&self, ports: &BoxedPorts) -> Result<JoinHandle<()>, ControllerBuilderError> {
        match (
            ports.get_float_source(&self.a),
            ports.get_float_source(&self.b),
            ports.get_float_source(&self.c),
            ports.get_float_source(&self.f),
            ports.get_float_sink(&self.x),
        ) {
            (Ok(a), Ok(b), Ok(c), Ok(f), Ok(x)) => {
                Ok(SecondOrderOde::new(a, b, c, f, x, self.x0, self.dx0, self.period_ms).handle)
            }
            (a, b, c, f, x) => {
                let mut errs = Vec::with_capacity(5);
                for x in [a, b, c, f] {
                    if let Err(e) = x {
                        errs.push(e)
                    }
                }
                if let Err(e) = x {
                    errs.push(e)
                }
                Err(ControllerBuilderError::from_errors(errs))
            }
        }
    }
}
