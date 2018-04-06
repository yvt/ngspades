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
/// ## Root `<Root> ::= <ElementInner>`
///
/// The root node contains the default attribute and zero or more items.
///
/// ## Item `<Item> ::= <Span> | <Element>`
///
/// Each item is one of the following:
///
/// ```text
/// <Span> ::= "#(" Expression ")" | "<" Type? ">(" Expression ")" `
/// ```
///
/// A span is denoted using one of the above three formats.
///
///  - `#(expr)`: `expr` is pushed into the resulting `TextBuf` directly.
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
/// containing elements, with those at a deeper level taking the precedence.
/// Multiple sets of attributes are combined using the trait [`Override`].
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
///         text! { default; #("Friendship ") {em; #("is")} #(" magic!") };
///
///     let owned: attrtext::TextBuf<String, Option<&str>> =
///         text! { default; <>("Friendship ") {em; <>("is")} <>(" magic!") };
///
///     assert_eq!(text.to_string(), "Friendship is magic!");
///     assert_eq!(owned.to_string(), "Friendship is magic!");
///     # }
///
#[macro_export]
macro_rules! text {
    ($attr:expr; $($rest:tt)*) => {{
        let mut text = $crate::TextBuf::new();
        {
            let _text = &mut text;
            text! { @push(_text, $attr) $($rest)* };
        }
        text
    }};

    (@push($text:expr, $attr:expr)) => {};
    (@push($text:expr, $attr:expr) #($span:expr) $($rest:tt)*) => {{
        $text.push($span, ::std::clone::Clone::clone(&$attr));

        text! { @push($text, $attr) $($rest)* };
    }};
    (@push($text:expr, $attr:expr) <$type:ty>($span:expr) $($rest:tt)*) => {{
        text! { @push($text, $attr) #(<$type as ::std::convert::From>::from($span)) };
        text! { @push($text, $attr) $($rest)* };
    }};
    (@push($text:expr, $attr:expr) <>($span:expr) $($rest:tt)*) => {{
        text! { @push($text, $attr) #(::std::convert::From::from($span)) };
        text! { @push($text, $attr) $($rest)* };
    }};
    (@push($text:expr, $attr:expr) {$newattr:expr; $($inner:tt)*} $($rest:tt)*) => {{
        let newattr = $crate::attr::Override::override_with(&$attr, &$newattr);
        text! { @push($text, newattr) $($inner)* };

        text! { @push($text, $attr) $($rest)* };
    }}
}
