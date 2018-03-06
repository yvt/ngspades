//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate itervalues;
use itervalues::IterValues;

#[test]
fn bools() {
    let values: Vec<_> = <bool>::iter_values().collect();
    assert_eq!(values.as_slice(), &[false, true]);
}

#[test]
fn bool_pairs() {
    let values: Vec<_> = <(bool, bool)>::iter_values().collect();
    assert_eq!(
        values.as_slice(),
        &[(false, false), (false, true), (true, false), (true, true)]
    );
}

#[test]
fn bools2() {
    let values: Vec<_> = <(bool,)>::iter_values().collect();
    assert_eq!(values.as_slice(), &[(false,), (true,)]);
}
