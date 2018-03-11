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
use spirv_cross::{ExecutionModel, SpirV2Msl};

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
    // Each arugment table index is directly mapped to Metal buffer index
    tables: Arc<Vec<Option<ArgTableSig>>>,
}

zangfx_impl_handle! { RootSig, handles::RootSig }

impl RootSig {
    pub(crate) fn setup_spirv2msl(&self, s2m: &mut SpirV2Msl, stage: ExecutionModel) {
        for (arg_table_index, table) in self.tables.iter().enumerate() {
            if let &Some(ref table) = table {
                table.setup_spirv2msl(
                    s2m,
                    arg_table_index as u32,
                    arg_table_index as u32,
                    stage,
                );
            }
        }
    }
}
