//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/// Work-around for the rust issue [#26925](https://github.com/rust-lang/rust/issues/26925).
macro_rules! derive_using_field {
    (($($gp:tt)*); PartialEq for $type:ty => $field:ident) => (
        impl<$($gp)*> PartialEq for $type {
            fn eq(&self, other: &Self) -> bool {
                PartialEq::eq(&self.$field, &other.$field)
            }
        }
    );
    (($($gp:tt)*); Eq for $type:ty => $field:ident) => (
        impl<$($gp)*> Eq for $type {}
    );
    (($($gp:tt)*); Hash for $type:ty => $field:ident) => (
        impl<$($gp)*> ::std::hash::Hash for $type {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                ::std::hash::Hash::hash(&self.$field, state)
            }
        }
    );
    (($($gp:tt)*); Clone for $type:ty => $field:ident) => (
        impl<$($gp)*> Clone for $type {
            fn clone(&self) -> Self {
                Self { $field: Clone::clone(&self.$field) }
            }
        }
    );
    (($($gp:tt)*); Debug for $type:ty => $field:ident) => (
        impl<$($gp)*> ::std::fmt::Debug for $type {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                // this is incorrect because $type includes generic parameters,
                // but shouldn't be a problem for a debug purpose
                f.debug_struct(stringify!($type))
                    .field(stringify!($field), &self.$field)
                    .finish()
            }
        }
    );

    (($($gp:tt)*); ($name:ident) for $type:ty => $field:ident) => (
        derive_using_field! { ($($gp)*); $name for $type => $field }
    );
    (($($gp:tt)*); ($name:ident, $($rest:ident),*) for $type:ty => $field:ident) => (
        derive_using_field! { ($($gp)*); $name for $type => $field }
        derive_using_field! { ($($gp)*); ($($rest),*) for $type => $field }
    )
}
