//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines internal utility macros.

/// Creates a `match` statement which includes an arm for each known constant.
/// The matched value is captured as a constant item (rather than a normal
/// variable), which can appear in a constant expression context.
///
/// This macro essentially converts an input expression into a constant. Thus,
/// it is useful for calling intrinsic functions such as `_mm_i32gather_epi32`
/// that only accept constant values for some parameter.
#[allow(dead_code)]
macro_rules! constantify {
    (
        match ($var:expr) {
            $x0:literal $(| $xn:literal)* @ $name:ident: $t:ty => $knownvalue:expr,
            _ => $elsevalue:expr,
        }
    ) => (
        match $var {
            $x0 => {
                #[allow(non_upper_case_globals)]
                const $name: $t = $x0;
                $knownvalue
            }
            $(
                $xn => {
                    #[allow(non_upper_case_globals)]
                    const $name: $t = $xn;
                    $knownvalue
                }
            ),*
            _ => $elsevalue,
        }
    )
}