//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `RootSig` for Metal.
use std::sync::Arc;

use base::{arg, handles, ArgTableIndex};
use common::Result;

use super::tablesig::ArgTableSig;

/// Implementation of `RootSigBuilder` for Metal.
#[derive(Debug)]
pub struct RootSigBuilder {
    tables: Vec<Option<ArgTableSig>>,
}

zangfx_impl_object! { RootSigBuilder: arg::RootSigBuilder, ::Debug }

impl RootSigBuilder {
    /// Construct an `RootSigBuilder`.
    pub fn new() -> Self {
        Self { tables: Vec::new() }
    }
}

impl arg::RootSigBuilder for RootSigBuilder {
    fn arg_table(
        &mut self,
        index: ArgTableIndex,
        x: &handles::ArgTableSig,
    ) -> &mut arg::RootSigBuilder {
        let our_table: &ArgTableSig = x.downcast_ref().expect("bad argument table signature type");
        if self.tables.len() <= index {
            self.tables.resize(index + 1, None);
        }
        self.tables[index] = Some(our_table.clone());
        self
    }

    fn build(&mut self) -> Result<handles::RootSig> {
        let root_sig = RootSig {
            tables: Arc::new(self.tables.clone()),
        };
        Ok(handles::RootSig::new(root_sig))
    }
}

/// Implementation of `RootSig` for Metal.
#[derive(Debug, Clone)]
pub struct RootSig {
    tables: Arc<Vec<Option<ArgTableSig>>>,
}

zangfx_impl_handle! { RootSig, handles::RootSig }
