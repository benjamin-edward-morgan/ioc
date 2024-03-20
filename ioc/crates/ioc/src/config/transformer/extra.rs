use std::collections::{HashMap, HashSet};

use super::TransformerConfig;
use ioc_core::{error::IocBuildError, InputKind, Transformer, TransformerI, Value};
use ioc_extra::transform::{
    hbridge::{HBridge, HBridgeConfig},
    linear::{LinearTransform, LinearTransformConfig},
    function::{FunctionTransformer},
    pid::{Pid, PidConfig},
    limiter::{LimiterParams, LimiterFilterConfig, Limiter},
    average::{WindowAverageFilterConfig, WindowAverage},
};

use serde::Deserialize;

///Creates a transformer that consumes a Float input that is positive or negative. Emits three inputs:
/// - forward - this is the input when it is positive, zero otherwise
/// - reverse - this is -input when it is negative, zero otherwise
/// - enable - this is 1.0 when input is nonzero, zero otherwize
#[derive(Deserialize, Debug)]
pub struct HBridgeTransformerConfig {
    input: String,
}

impl TransformerConfig for HBridgeTransformerConfig {
    async fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> Result<TransformerI, IocBuildError> {
        let input = match upstream_inputs.get(&self.input) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build hbridge from non-float input of type {x:?}"
                )))
            }
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.input
                )))
            }
        };
        let cfg = HBridgeConfig { input };
        let hbridge = HBridge::try_build(&cfg).await?;

        Ok(TransformerI {
            join_handle: hbridge.join_handle,
            inputs: HashMap::from([
                ("forward".to_owned(), InputKind::float(hbridge.forward)),
                ("reverse".to_owned(), InputKind::float(hbridge.reverse)),
                ("enable".to_owned(), InputKind::float(hbridge.enable)),
            ]),
        })
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        HashSet::from([&self.input])
    }
}

///Emits a linear transformer that consumes from a Float input, and emits an input named 'value' 
/// from must contain two values for the domain. from[0] must be less than from[1]
/// to must contain two values for range. if to[0] > to[1] then there will be an inverse relationship. 
/// If the input supplied is beyond the domain, the output emitted will be beyond the range.
#[derive(Deserialize, Debug)]
pub struct LinearTransformerConfig {
    input: String,
    from: Vec<f64>,
    to: Vec<f64>,
}

impl TransformerConfig for LinearTransformerConfig {
    async fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> Result<TransformerI, IocBuildError> {
        if self.from.len() != 2 || self.to.len() != 2 {
            return Err(IocBuildError::message(
                "LinearTransform must have exactly two values for the 'from' and 'to' fields",
            ));
        }
        let input = match upstream_inputs.get(&self.input) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build linear transformer from non-float input of type {x:?}"
                )))
            }
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.input
                )))
            }
        };
        let lcfg = LinearTransformConfig::from_ranges(
            input,
            &[self.from[0], self.from[1]],
            &[self.to[0], self.to[1]],
        )?;
        let xform = LinearTransform::try_build(&lcfg).await?;
        Ok(TransformerI {
            join_handle: xform.join_handle,
            inputs: HashMap::from([("value".to_string(), InputKind::float(xform.value))]),
        })
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        HashSet::from([&self.input])
    }
}


///Clamps the input Float to the min and max, inclusive. Emits an inputs called 'value'
#[derive(Debug,Deserialize)]
pub struct ClampConfig {
    pub input: String,
    pub min: f64,
    pub max: f64,
}

impl TransformerConfig for ClampConfig {
    async fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> Result<TransformerI, IocBuildError> {
        if self.min > self.max {
            return Err(IocBuildError::from_string(format!(
                "unable to build clamp transformer. must have min <= max"
            )));
        }
        let input = match upstream_inputs.get(&self.input) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build clamp transformer from non-float input of type {:?}",
                    x
                )));
            }
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.input
                )));
            }
        };
        let min = self.min;
        let max = self.max;
        let output = FunctionTransformer::new(input, move |x: f64| {
            x.min(max).max(min)
        });
        Ok(TransformerI{
            join_handle: output.join_handle,
            inputs: HashMap::from([
                ("value".to_string(), InputKind::float(output.value)),
            ])
        })
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        HashSet::from([&self.input])
    }
}

///Consumes an Array input, expected to be a 3-vector. Drops the z axis and computes atan2(y,x) and emits an input named 'value'
#[derive(Debug,Deserialize)]
pub struct HeadingConfig {
    pub input: String,
}

impl TransformerConfig for HeadingConfig {
    async fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> Result<TransformerI, IocBuildError> {

        let input = match upstream_inputs.get(&self.input) {
            Some(InputKind::Array(arr)) => arr.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build clamp transformer from non-array input of type {:?}",
                    x
                )));
            },
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.input
                )));
            }
        };

        let output = FunctionTransformer::new(input, move |vec: Vec<Value>| {
            if vec.len() > 2 {
                let Value::Float(x) = vec.get(0).unwrap();
                let Value::Float(y) = vec.get(1).unwrap();
                y.atan2(*x)
            } else {
                f64::NAN
            }
        });
        Ok(TransformerI{
            join_handle: output.join_handle,
            inputs: HashMap::from([
                ("value".to_string(), InputKind::float(output.value)),
            ])
        })
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        HashSet::from([&self.input])
    }
}

///Configuration for a tunable PID controller. 
/// p, i, and d are inputs for the P, I, and D coefficients respectively. Integrals and derivatives are calculated numerically.
/// set_point is the desired state
/// process_var is the observed state
/// period_ms is the number of milliseconds between frames
/// emits an input named 'value' which is the control signal.
#[derive(Debug,Deserialize)]
pub struct PidCtrlConfig {
    p: String,
    i: String,
    d: String,
    set_point: String,
    process_var: String,
    period_ms: u16,
}

impl TransformerConfig for PidCtrlConfig {
    async fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> Result<TransformerI, IocBuildError> {
        let p = match upstream_inputs.get(&self.p) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build pid controller from non-float input of type {:?}",
                    x
                )));
            }
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.p
                )));
            }
        };
        let i = match upstream_inputs.get(&self.i) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build pid controller from non-float input of type {:?}",
                    x
                )));
            }
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.i
                )));
            }
        };
        let d = match upstream_inputs.get(&self.d) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build pid controller from non-float input of type {:?}",
                    x
                )));
            }
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.d
                )));
            }
        };
        let sp = match upstream_inputs.get(&self.set_point) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build pid controller from non-float input of type {:?}",
                    x
                )));
            }
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.set_point
                )));
            }
        };
        let pv = match upstream_inputs.get(&self.process_var) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build pid controller from non-float input of type {:?}",
                    x
                )));
            }
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.process_var
                )));
            }
        };

        let pid = Pid::try_build(
            &PidConfig{
                set_point: sp,
                process_var: pv,
                p: p,
                i: i,
                d: d,
                period_ms: self.period_ms,
            }
        ).await?;

        Ok(TransformerI{
            join_handle: pid.join_handle,
            inputs: HashMap::from([
                ("value".to_string(), InputKind::float(pid.value)),
            ])
        })
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        HashSet::from([
            &self.p,
            &self.i,
            &self.d,
            &self.process_var,
            &self.set_point,
        ])
    }
}

///Keeps an internal 'position' var and moves it with limited velocity and acceleration to the input.
/// min/max are the range of the output
/// vmin/vmax are limits on the velocity in units/sec. Must have vmin < 0 and vmax > 0
/// amin/amax are the acceleration values used to move. Must have amin < 0 and amax > 0
/// emits an input named 'value' every period_ms milliseconds
#[derive(Debug, Deserialize)]
pub struct LimiterConfig {
    input: String,
    amin: f64,
    amax: f64,
    vmin: f64,
    vmax: f64,
    min: f64,
    max: f64,
    period_ms: u16,
}

impl TransformerConfig for LimiterConfig {
    async fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> Result<TransformerI, IocBuildError> {
        let input = match upstream_inputs.get(&self.input) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build limiter from non-float input of type {:?}",
                    x
                )));
            }
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.input
                )));
            }
        };
        let cfg = LimiterFilterConfig{
            input,
            params: LimiterParams{
                min: self.min, max: self.max,
                dmin: self.vmin, dmax: self.vmax,
                ddmin: self.amin, ddmax: self.amax,
                period_ms: self.period_ms as u64,
            }
        };
        let limiter = Limiter::try_build(&cfg).await?;

        Ok(TransformerI{
            join_handle: limiter.join_handle,
            inputs: HashMap::from([
                ("value".to_string(), InputKind::float(limiter.value)),
            ])
        })
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        HashSet::from([
            &self.input
        ])
    }
}

///This takes fast-changing input Float value, calculates its average using reimann sums over windows period_ms milliseconds and emits those average values.
#[derive(Debug, Deserialize)]
pub struct WindowAverageConfig {
    input: String,
    period_ms: u64,
}

impl TransformerConfig for WindowAverageConfig {
    async fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> Result<TransformerI, IocBuildError> {
        let input = match upstream_inputs.get(&self.input) {
            Some(InputKind::Float(float)) => float.as_ref(),
            Some(x) => {
                return Err(IocBuildError::from_string(format!(
                    "unable to build windowed average filter from non-float input of type {:?}",
                    x
                )));
            }
            None => {
                return Err(IocBuildError::from_string(format!(
                    "no input with name {}",
                    self.input
                )));
            }
        };
        let cfg = WindowAverageFilterConfig{
            input,
            period_ms: self.period_ms,
        };
        let avg = WindowAverage::try_build(&cfg).await?;

        Ok(TransformerI{
            join_handle: avg.join_handle,
            inputs: HashMap::from([
                ("value".to_string(), InputKind::float(avg.value)),
            ])
        })
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        HashSet::from([
            &self.input
        ])
    }
}