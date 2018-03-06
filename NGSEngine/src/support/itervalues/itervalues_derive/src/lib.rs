//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Procedural macro for [`itervalues`](../itervalues/index.html).
#![recursion_limit = "2048"]
extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use syn::{Data, DataEnum, DeriveInput, Fields, Ident};
use quote::{ToTokens, Tokens};
use proc_macro::TokenStream;

#[proc_macro_derive(IterValues, attributes(IterValues))]
pub fn derive_iter_all_values(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    if ast.generics.params.len() > 0 {
        panic!("`derive(IterValues)` does not generics (yet)");
    }

    let quote_tokens = match ast.data {
        Data::Enum(ref data) => gen_enum(&ast.ident, &ast, data),
        _ => panic!("`derive(IterValues)` may only be applied to enums (currently)"),
    };

    // println!("{:?}", quote_tokens);
    quote_tokens.into()
}

fn gen_enum(ident: &Ident, item: &DeriveInput, data: &DataEnum) -> Tokens {
    let fieldless = data.variants.iter().all(|v| match v.fields {
        Fields::Unit => true,
        Fields::Named(ref fields) => fields.named.len() == 0,
        Fields::Unnamed(ref fields) => fields.unnamed.len() == 0,
    });

    if fieldless {
        // Use a static value list
        let var_exprs = data.variants.iter().map(|v| {
            let ref v_ident = v.ident;
            match v.fields {
                Fields::Unit => quote! { #ident::#v_ident },
                Fields::Named(_) => quote! { #ident::#v_ident {} },
                Fields::Unnamed(_) => quote! { #ident::#v_ident () },
            }
        });
        return quote! {
            impl ::itervalues::IterValues for #ident {
                type Iterator = ::std::iter::Cloned<::std::slice::Iter<'static, Self>>;

                fn iter_values() -> Self::Iterator {
                    [#(#var_exprs),*].into_iter().cloned()
                }
            }
        };
    }

    let state_name = Ident::from(format!("{}IterValues", ident));

    // List containing each variant and `Tokens` of a tuple type that represents
    // values contained in the variant, like `(T1, (T2, (T3,)))`.
    let variants_and_types: Vec<_> = data.variants
        .iter()
        .map(|variant| {
            let fields = match variant.fields {
                Fields::Unit => return (variant, None),
                Fields::Named(ref fields) => &fields.named,
                Fields::Unnamed(ref fields) => &fields.unnamed,
            };

            if fields.len() == 0 {
                return (variant, None);
            }

            // Since `Punctuated` does not implement `DoubleEndedIterator`...
            let fields: Vec<_> = fields.iter().collect();
            let value_tuple = fields.iter().rev().fold(quote!{}, |second, field| {
                let ref ty = field.ty;
                quote! { (#ty, #second) }
            });

            (variant, Some(value_tuple))
        })
        .collect();

    let state_variants = variants_and_types
        .iter()
        .map(|&(variant, ref value_tuple)| {
            let ref variant_ident = variant.ident;

            if value_tuple.is_none() {
                // Field-less(-like)
                return variant_ident.into_tokens();
            }

            quote! {
                #variant_ident(<#value_tuple as ::itervalues::IterValues>::Iterator)
            }
        });

    // List containing an expression for starting each state
    let state_initializers: Vec<_> = variants_and_types
        .iter()
        .map(|&(variant, ref value_tuple)| {
            let ref variant_ident = variant.ident;

            if value_tuple.is_none() {
                // Field-less(-like)
                return quote! {
                    #state_name::#variant_ident
                };
            }

            quote! {
                #state_name::#variant_ident(
                    <#value_tuple as ::itervalues::IterValues>::iter_values())
            }
        })
        .collect();

    // `match` case for each state. (``)
    let state_cases = variants_and_types.iter().enumerate().map(
        |(i, &(variant, ref value_tuple))| {
            let ref v_ident = variant.ident;

            let next_state = if let Some(init) = state_initializers.get(i + 1) {
                quote! { Some(#init) }
            } else {
                quote! { Some(#state_name::__IterValuesEnd) }
            };

            if value_tuple.is_none() {
                // Field-less(-like)

                let value = match variant.fields {
                    Fields::Unit => quote! { #ident::#v_ident },
                    Fields::Named(_) => quote! { #ident::#v_ident {} },
                    Fields::Unnamed(_) => quote! { #ident::#v_ident () },
                };

                return quote! {
                    #state_name::#v_ident => (#next_state, Some(#value))
                };
            }

            let num_fields = match variant.fields {
                Fields::Unit => unreachable!(),
                Fields::Named(ref fields) => &fields.named,
                Fields::Unnamed(ref fields) => &fields.unnamed,
            }.len();

            // An expression that refers each field in `value`,
            // e.g., `((value.1).1).0`
            let field_values = (0..num_fields).map(|i| {
                let t = (0..i).fold(quote!{ value }, |inner, _| {
                    quote! { (#inner).1 }
                });
                quote! { (#t).0 }
            });

            // A `#ident` value
            let expanded_value = match variant.fields {
                Fields::Unit => unimplemented!(),
                Fields::Named(ref fields) => {
                    let item = field_values.zip(fields.named.iter()).map(|(value, field)| {
                        let ref field_ident = field.ident;
                        quote! { #field_ident: #value }
                    });
                    quote! { #ident::#v_ident {
                        #(#item),*
                    } }
                }
                Fields::Unnamed(_) => quote! { #ident::#v_ident (
                    #(#field_values),*
                ) },
            };

            return quote! {
                #state_name::#v_ident(ref mut it) => {
                    if let Some(value) = it.next() {
                        (None, Some(#expanded_value))
                    } else {
                        (#next_state, None)
                    }
                }
            };
        },
    );

    let ref vis = item.vis;
    let ref start = state_initializers[0];

    quote! {
        #[doc(hidden)]
        #vis enum #state_name {
            #(#state_variants,)*
            __IterValuesEnd
        }

        impl ::std::iter::Iterator for #state_name {
            type Item = #ident;

            fn next(&mut self) -> ::std::option::Option<Self::Item> {
                loop {
                    let (next, value) = match *self {
                        #(#state_cases,)*
                        #state_name::__IterValuesEnd => {
                            return None;
                        },
                    };
                    if let Some(next) = next {
                        *self = next;
                        if value.is_some() {
                            // Transitioning to a new state at the same time of
                            // returning a value
                            return value;
                        }
                        // Did not get a value this time, but we got a new state
                    } else {
                        // The state did not change, or we are at the end
                        return value;
                    }
                }
            }
        }

        impl ::itervalues::IterValues for #ident {
            type Iterator = #state_name;

            fn iter_values() -> Self::Iterator {
                #start
            }
        }
    }
}
