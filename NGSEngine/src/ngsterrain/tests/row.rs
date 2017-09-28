//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngsterrain;

use ngsterrain::*;

#[test]
fn random() {
    let voxels = [
        None,
        None,
        Some(SolidVoxel::Colored(ColoredVoxel::new([50, 60, 70, 80]))),
        Some(SolidVoxel::Colored(ColoredVoxel::new([51, 61, 71, 81]))),
        Some(SolidVoxel::Uncolored),
        Some(SolidVoxel::Uncolored),
        Some(SolidVoxel::Uncolored),
        Some(SolidVoxel::Colored(ColoredVoxel::new([52, 62, 72, 82]))),
        Some(SolidVoxel::Colored(ColoredVoxel::new([53, 63, 73, 83]))),
        None,
        None,
        Some(SolidVoxel::Colored(ColoredVoxel::new([51, 61, 71, 81]))),
        Some(SolidVoxel::Uncolored),
        Some(SolidVoxel::Uncolored),
        Some(SolidVoxel::Uncolored),
        Some(SolidVoxel::Colored(ColoredVoxel::new([52, 62, 72, 82]))),
        None,
        None,
    ];
    let mut v = Vec::new();
    let mut row = Row::new(voxels.len(), &mut v);
    row.update_with(voxels.iter().map(Clone::clone)).unwrap();
    println!("row: {:?}", row);
    println!("chunks:");
    {
        let mut chunks = row.chunks();
        while let Some(chunk) = chunks.next() {
            println!("    - {:?}", &chunk.collect::<Vec<_>>());
        }
    }
    row.validate().unwrap();
    for (i, voxel) in voxels.iter().enumerate() {
        assert_eq!(
            row.get_voxel(i).unwrap().map(|sv| sv.into_owned()),
            voxels[i],
            "[{:?}] = {:?}",
            i,
            voxel
        );
    }
}
