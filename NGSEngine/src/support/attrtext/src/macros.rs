//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/// Constructs a `TextBuf` value.
///
/// # Syntax
///
/// The syntax of this macro is defined as below:
///
/// ## Root `<Root> ::= <Element>*`
///
/// The root node contains zero or more elements. A span cannot appear here
/// because an attribute is not defined here.
///
/// ## Item `<Item> ::= <Span> | <Element>`
///
/// Each item is one of the following:
///
/// ```text
/// <Span> ::= ("<" Type? ">")? "(" Expression ")" `
/// ```
///
/// A span is denoted using one of the following three formats.
///
///  - `(expr)`: `expr` is pushed into the resulting `TextBuf` directly.
///  - `<Type>(expr)`: `expr` is converted to `Type` using the
///    `std::convert::From` trait.
///  - `<>(expr)`: Similar to the above, but doesn't explicitly specify the type.
///
/// ```text
/// <Element> ::= "{" <ElementInner> "}"
/// <ElementInner> ::= Expression ";" <Item>*
/// ```
///
/// An element is composed of an expression specifying the attribute and zero
/// or more child items. The child items inherit the attributes of their
/// containing elements. Attributes from a containing element are combined using
/// the trait [`Override`].
///
/// [`Override`]: Override
///
/// # Examples
///
///     #[macro_use]
///     extern crate attrtext;
///     # fn main() {
///     let default = None;
///     let em = Some("emphasize");
///
///     let text: attrtext::TextBuf<&str, Option<&str>> =
///         text! {{ default; ("Friendship ") {em; ("is")} (" magic!") }};
///
///     let owned: attrtext::TextBuf<String, Option<&str>> =
///         text! {{ default; <>("Friendship ") {em; <>("is")} <>(" magic!") }};
///
///     let flat: attrtext::TextBuf<&str, Option<&str>> = text! {
///         { default; ("Friendship ") }
///         { em; ("is") }
///         { default; (" magic!") }
///     };
///
///     assert_eq!(text.to_string(), "Friendship is magic!");
///     assert_eq!(owned.to_string(), "Friendship is magic!");
///     assert_eq!(flat, text);
///     # }
///
#[macro_export]
macro_rules! text {
    ($({$($rest:tt)*})*) => {{
        let mut text = $crate::TextBuf::new();
        {
            let _text = &mut text;
            text! { @root(_text) $({$($rest)*})* };
        }
        text
    }};

    (@push($text:expr, $attr:expr)) => {};
    (@root($text:expr)) => {};
    (@push($text:expr, $attr:expr) ($span:expr) $($rest:tt)*) => {{
        $text.push($span, ::std::clone::Clone::clone(&$attr));

        text! { @push($text, $attr) $($rest)* };
    }};
    (@push($text:expr, $attr:expr) <$type:ty>($span:expr) $($rest:tt)*) => {{
        text! { @push($text, $attr) (<$type as ::std::convert::From>::from($span)) };
        text! { @push($text, $attr) $($rest)* };
    }};
    (@push($text:expr, $attr:expr) <>($span:expr) $($rest:tt)*) => {{
        text! { @push($text, $attr) (::std::convert::From::from($span)) };
        text! { @push($text, $attr) $($rest)* };
    }};
    (@root($text:expr) {$newattr:expr; $($inner:tt)*} $($rest:tt)*) => {{
        text! { @push($text, $newattr) $($inner)* };

        text! { @root($text) $($rest)* };
    }};
    (@push($text:expr, $attr:expr) {$newattr:expr; $($inner:tt)*} $($rest:tt)*) => {{
        let newattr = $crate::attr::Override::override_with(&$attr, &$newattr);
        text! { @push($text, newattr) $($inner)* };

        text! { @push($text, $attr) $($rest)* };
    }}
}
