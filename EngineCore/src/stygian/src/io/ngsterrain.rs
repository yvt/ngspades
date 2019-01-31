//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsTerrain interop
use crate::terrain::{Terrain, TerrainLevel};

fn level_from_ngsterrain(nt_terrain: &ngsterrain::Terrain) -> TerrainLevel {
    let size = nt_terrain.size();

    assert!(size.x.is_power_of_two() || size.y.is_power_of_two());

    let mut level = TerrainLevel {
        rows: vec![vec![]; size.x * size.y],
    };

    for (xy, nt_row) in nt_terrain.rows() {
        level.rows[xy.x + xy.y * size.x]
            .extend(nt_row.chunk_z_ranges().map(|x| x.start as _..x.end as _));
    }

    level
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum NgsTerrainConversionError {
    UnsupportedSize,
}

impl Terrain {
    pub fn from_ngsterrain(
        nt_terrain: &ngsterrain::Terrain,
    ) -> Result<Self, NgsTerrainConversionError> {
        let size = nt_terrain.size();

        if !(size.x.is_power_of_two() && size.x.is_power_of_two() && size.x.is_power_of_two()) {
            return Err(NgsTerrainConversionError::UnsupportedSize);
        }

        let base_level = level_from_ngsterrain(nt_terrain);

        Ok(Terrain::from_base_level(size, base_level))
    }
}
