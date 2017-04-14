//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{KernelType, KernelCreationParams};
use std::fmt;

pub trait StaticParams : fmt::Debug + 'static {
    fn inverse(&self) -> bool;
    fn kernel_type(&self) -> KernelType;
    fn check_param(&self, cparams: &KernelCreationParams) {
        assert_eq!(cparams.inverse, self.inverse());
        assert_eq!(cparams.kernel_type, self.kernel_type());
    }
}

#[derive(Debug)] struct StaticParamsDitForward {}
impl StaticParams for StaticParamsDitForward {
    #[inline] fn inverse(&self) -> bool { false }
    #[inline] fn kernel_type(&self) -> KernelType { KernelType::Dit }
}

#[derive(Debug)] struct StaticParamsDitBackward {}
impl StaticParams for StaticParamsDitBackward {
    #[inline] fn inverse(&self) -> bool { true }
    #[inline] fn kernel_type(&self) -> KernelType { KernelType::Dit }
}

#[derive(Debug)] struct StaticParamsDifForward {}
impl StaticParams for StaticParamsDifForward {
    #[inline] fn inverse(&self) -> bool { false }
    #[inline] fn kernel_type(&self) -> KernelType { KernelType::Dif }
}

#[derive(Debug)] struct StaticParamsDifBackward {}
impl StaticParams for StaticParamsDifBackward {
    #[inline] fn inverse(&self) -> bool { true }
    #[inline] fn kernel_type(&self) -> KernelType { KernelType::Dif }
}

/// Poor man's generic lambda
pub trait StaticParamsConsumer<TRet> {
    fn consume<T: StaticParams>(self, cparams: &KernelCreationParams, sparams: T) -> TRet;
}

pub fn branch_on_static_params<F, T>(cparams: &KernelCreationParams, f: F) -> T
    where F : StaticParamsConsumer<T> {

    match (cparams.kernel_type, cparams.inverse) {
        (KernelType::Dit, false) => f.consume(cparams, StaticParamsDitForward{}),
        (KernelType::Dif, false) => f.consume(cparams, StaticParamsDifForward{}),
        (KernelType::Dit, true) =>  f.consume(cparams, StaticParamsDitBackward{}),
        (KernelType::Dif, true) =>  f.consume(cparams, StaticParamsDifBackward{})
    }
}
