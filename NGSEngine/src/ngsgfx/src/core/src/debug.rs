//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

pub trait Marker {
    fn set_label(&self, label: Option<&str>) {}
}
