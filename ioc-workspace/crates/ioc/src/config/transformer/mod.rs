pub mod core;

#[cfg(feature = "extra")]
pub mod extra;

#[cfg(feature = "extra")]
use extra::{HBridgeTransformerConfig,LinearTransformerConfig};

use core::SumTransformerConfig;
use std::future::Future;
use std::collections::{HashMap, HashSet};
use ioc_core::{error::IocBuildError, InputKind, TransformerI};
use serde::Deserialize;

pub trait TransformerConfig {
    fn try_build(&self,  upstream_inputs: &HashMap<String, InputKind>) -> impl Future<Output=Result<TransformerI, IocBuildError>>;
    fn needs_inputs(&self) -> HashSet<&String>;
}

#[derive(Deserialize,Debug)]
pub enum IocTransformerConfig {
    //core
    Sum(SumTransformerConfig),

    //extra 
    #[cfg(feature = "extra")]
    HBridge(HBridgeTransformerConfig),
    #[cfg(feature = "extra")]
    LinearTransform(LinearTransformerConfig)
}

impl IocTransformerConfig {
    pub async fn try_build(&self, upstream_inputs: &HashMap<String, InputKind>) -> Result<TransformerI, IocBuildError> {
        match self {
            //core
            Self::Sum(sum) => sum.try_build(upstream_inputs).await,

            //extra 
            #[cfg(feature = "extra")]
            Self::HBridge(hbridge) => hbridge.try_build(upstream_inputs).await,

            #[cfg(feature = "extra")]
            Self::LinearTransform(lxform) => lxform.try_build(upstream_inputs).await,
        }
    }

    pub fn needs_inputs(&self) -> HashSet<&String> {
        match self {
            //core
            Self::Sum(sum) => sum.needs_inputs(),

            //extra 
            #[cfg(feature = "extra")]
            Self::HBridge(hbridge) => hbridge.needs_inputs(),

            #[cfg(feature = "extra")]
            Self::LinearTransform(lxform) => lxform.needs_inputs(),
        }
    }
}