//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::borrow::Borrow;
use super::{Setup, Num};
use num_traits::Zero;
use super::kernel::{KernelParams};
use std::mem;
use num_complex::Complex;

#[derive(Debug)]
pub struct Env<TNum, TSetupRef> {
    setup: TSetupRef,
    work_area: Vec<TNum>
}

impl<TNum, TSetupRef> Env<TNum, TSetupRef>
    where TNum: Num + 'static,
          TSetupRef: Borrow<Setup<TNum>> {

    pub fn new(setup: TSetupRef) -> Self {
        let work_area_size = setup.borrow().required_work_area_size();
        Env {
            setup: setup,
            work_area: vec![Zero::zero(); work_area_size]
        }
    }

    pub fn transform(&mut self, data: &mut [TNum]) {
        let mut kernel_param = KernelParams {
            coefs: data,
            work_area: self.work_area.as_mut_slice()
        };
        let setup = self.setup.borrow();
        // println!("{:#?}", setup);
        // println!("{:#?}", &unsafe{::std::mem::transmute::<&[TNum], &[::num_complex::Complex<TNum>]>(kernel_param.coefs)}[0 .. kernel_param.coefs.len() / 2]);
        for kernel in &setup.kernels {
            kernel.transform(&mut kernel_param);
            // println!("{:#?}", &unsafe{::std::mem::transmute::<&[TNum], &[::num_complex::Complex<TNum>]>(kernel_param.coefs)}[0 .. kernel_param.coefs.len() / 2]);
        }
    }
}
