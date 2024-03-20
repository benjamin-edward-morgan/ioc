pub mod core;

#[cfg(feature = "extra")]
pub mod extra;

#[cfg(feature = "extra")]
use extra::{HBridgeTransformerConfig, LinearTransformerConfig, ClampConfig, HeadingConfig, PidCtrlConfig, LimiterConfig, WindowAverageConfig};

use core::SumTransformerConfig;
use ioc_core::{error::IocBuildError, InputKind, TransformerI};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::future::Future;

pub trait TransformerConfig {
    fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> impl Future<Output = Result<TransformerI, IocBuildError>>;
    fn needs_inputs(&self) -> HashSet<&String>;
}

///All possible objects that could appear below the `transformers` secion in the config file.
#[derive(Deserialize, Debug)]
pub enum IocTransformerConfig {
    //core
    Sum(SumTransformerConfig),

    //extra
    #[cfg(feature = "extra")]
    HBridge(HBridgeTransformerConfig),
    #[cfg(feature = "extra")]
    LinearTransform(LinearTransformerConfig),
    #[cfg(feature = "extra")]
    Clamp(ClampConfig),
    #[cfg(feature = "extra")]
    Heading(HeadingConfig),
    #[cfg(feature = "extra")]
    PID(PidCtrlConfig),
    #[cfg(feature = "extra")]
    Limiter(LimiterConfig),
    #[cfg(feature = "extra")]
    WindowAverage(WindowAverageConfig),
}

impl IocTransformerConfig {
    //Attempts to build this transformer given a map of all named upstream inputs.
    pub async fn try_build(
        &self,
        upstream_inputs: &HashMap<String, InputKind>,
    ) -> Result<TransformerI, IocBuildError> {
        match self {
            //core
            Self::Sum(sum) => sum.try_build(upstream_inputs).await,

            //extra
            #[cfg(feature = "extra")]
            Self::HBridge(hbridge) => hbridge.try_build(upstream_inputs).await,
            #[cfg(feature = "extra")]
            Self::LinearTransform(lxform) => lxform.try_build(upstream_inputs).await,
            #[cfg(feature = "extra")]
            Self::Clamp(clampcfg) => clampcfg.try_build(upstream_inputs).await,
            #[cfg(feature = "extra")]
            Self::Heading(hdgcfg) => hdgcfg.try_build(upstream_inputs).await,
            #[cfg(feature = "extra")]
            Self::PID(pidcfg) => pidcfg.try_build(upstream_inputs).await,
            #[cfg(feature = "extra")]
            Self::Limiter(limcfg) => limcfg.try_build(upstream_inputs).await,
            #[cfg(feature = "extra")]
            Self::WindowAverage(avgcfg) => avgcfg.try_build(upstream_inputs).await,
        }
    }

    //Returns the names of upstream inputs this Transformer will require to be built.
    pub fn needs_inputs(&self) -> HashSet<&String> {
        match self {
            //core
            Self::Sum(sum) => sum.needs_inputs(),

            //extra
            #[cfg(feature = "extra")]
            Self::HBridge(hbridge) => hbridge.needs_inputs(),
            #[cfg(feature = "extra")]
            Self::LinearTransform(lxform) => lxform.needs_inputs(),
            #[cfg(feature = "extra")]
            Self::Clamp(clampcfg) => clampcfg.needs_inputs(),
            #[cfg(feature = "extra")]
            Self::Heading(hdgcfg) => hdgcfg.needs_inputs(),
            #[cfg(feature = "extra")]
            Self::PID(pidcfg) => pidcfg.needs_inputs(),
            #[cfg(feature = "extra")]
            Self::Limiter(limcfg) => limcfg.needs_inputs(),
            #[cfg(feature = "extra")]
            Self::WindowAverage(avgcfg) => avgcfg.needs_inputs(),
        }
    }
}
