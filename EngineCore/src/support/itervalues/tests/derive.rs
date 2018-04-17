//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate itervalues;
#[macro_use]
extern crate itervalues_derive;
use itervalues::IterValues;

#[test]
fn fieldless() {
    #[derive(IterValues, Copy, Clone, PartialEq, Eq, Debug)]
    enum Test {
        A,
        B,
    }

    let values: Vec<_> = Test::iter_values().collect();
    assert_eq!(values.as_slice(), &[Test::A, Test::B]);
}

#[test]
fn fieldless_like() {
    #[derive(IterValues, Copy, Clone, PartialEq, Eq, Debug)]
    enum Test {
        A {},
        B(),
    }

    let values: Vec<_> = Test::iter_values().collect();
    assert_eq!(values.as_slice(), &[Test::A {}, Test::B()]);
}

#[test]
fn nested() {
    #[derive(IterValues, Copy, Clone, PartialEq, Eq, Debug)]
    enum Test1 {
        A,
        B,
    }

    #[derive(IterValues, Copy, Clone, PartialEq, Eq, Debug)]
    enum Test2 {
        X { a: Test1, b: bool },
        Y(Test1, bool),
        Z {},
    }

    let values: Vec<_> = Test2::iter_values().collect();
    assert_eq!(
        values.as_slice(),
        &[
            Test2::X {
                a: Test1::A,
                b: false,
            },
            Test2::X {
                a: Test1::A,
                b: true,
            },
            Test2::X {
                a: Test1::B,
                b: false,
            },
            Test2::X {
                a: Test1::B,
                b: true,
            },
            Test2::Y(Test1::A, false),
            Test2::Y(Test1::A, true),
            Test2::Y(Test1::B, false),
            Test2::Y(Test1::B, true),
            Test2::Z {}
        ]
    );
}
