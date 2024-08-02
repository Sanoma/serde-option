//! Code adapted from the `serde_with_macros` library:
//! <https://docs.rs/serde_with_macros/3.8.3/src/serde_with_macros/lib.rs.html>
//! which is licensed under the MIT license.
//!
//! Copyright (c) 2015
//! Permission is hereby granted, free of charge, to any
//! person obtaining a copy of this software and associated
//! documentation files (the "Software"), to deal in the
//! Software without restriction, including without
//! limitation the rights to use, copy, modify, merge,
//! publish, distribute, sublicense, and/or sell copies of
//! the Software, and to permit persons to whom the Software
//! is furnished to do so, subject to the following
//! conditions:
//!
//! The above copyright notice and this permission notice
//! shall be included in all copies or substantial portions
//! of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
//! ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
//! TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
//! PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
//! SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
//! CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
//! OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
//! IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
//! DEALINGS IN THE SOFTWARE.

use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_quote, spanned::Spanned, AngleBracketedGenericArguments, Error, Field, Fields,
    GenericArgument, ItemEnum, ItemStruct, PathArguments, QSelf, Type, TypeGroup, TypeParen,
    TypePath,
};

/// Process `#[nullable]` and `#[not_required]` annotations in [`Option`] fields and
/// convert them to their corresponding `#[serde(...)]` annotations.
///
/// JSON APIs sometimes have optional fields, as well as nullable fields.
/// This distinction matters at the schema level. However, [`serde`]'s own APIs
/// make this non-obvious and boilerplatey. This macro fixes that by exposing
/// two attributes `#[nullable]` and `#[not_required]` for ergonomic use.
///
/// This macro also respects the `#[serde(skip)]` and `#[serde(default)]` attributes
/// when processing.
///
/// # Example
///
/// ```
/// # use serde::Serialize;
/// # use serde_option_macros::serde_option;
/// #[serde_option] // <-- Put this before the #[derive]
/// #[derive(Serialize)]
/// struct Data {
///     #[nullable]
///     nullable_field: Option<String>,
///     #[not_required]
///     not_required_field: Option<u64>,
///     #[nullable]
///     #[not_required]
///     nullable_and_not_required_field: Option<Option<String>>,
///     #[nullable]
///     #[serde(default)]
///     nullable_with_default: Option<String>,
///     #[serde(skip)]
///     skipped_field: Option<bool>,
/// }
/// ```
///
/// This is equivalent to the following struct definition:
///
/// ```
/// # use serde::Serialize;
/// # use serde_option_macros::serde_option;
/// #[derive(Serialize)]
/// struct Data {
///     #[serde(with = "Option")]
///     nullable_field: Option<String>,
///     #[serde(default, skip_serializing_if = "Option::is_none")]
///     #[serde(with = "serde_with::rust::unwrap_or_skip")]
///     not_required_field: Option<u64>,
///     #[serde(default, skip_serializing_if = "Option::is_none")]
///     #[serde(with = "serde_with::rust::double_option")]
///     nullable_and_not_required_field: Option<Option<String>>,
///     #[serde(default)]
///     nullable_with_default: Option<String>,
///     #[serde(skip)]
///     skipped_field: Option<bool>,
/// }
/// ```
///
/// # Features
///
/// When compiling with the `utoipa` feature, this will also add
/// `#[schema(required = true)]` to required + nullable fields, and
/// `#[schema(schema_with = ...)]` to optional + non-nullable fields.
///
///
/// # Limitations
///
/// You must have the [`serde_with`] crate installed for the expansion to work.
///
/// Certain combinations of attributes are invalid and will raise a compile error:
/// * Using either `#[nullable]` or `#[not_required]` together with `#[serde(skip)]`
/// * Using `#[serde(default)]` with `#[not_required]`
///
/// The [`macro@serde_option`] only works if the type is called `Option`,
/// `std::option::Option`, or `core::option::Option`. Type aliasing an [`Option`] and giving it
/// another name, will cause a compile error. This cannot be supported, as proc-macros run
/// before type checking, thus it is not possible to determine if a type alias refers to an
/// [`Option`].
///
/// ```compile_fail
/// # use serde::Serialize;
/// # use serde_option_macros::serde_option;
/// type MyOption<T> = Option<T>;
///
/// #[serde_option]
/// #[derive(Serialize)]
/// struct Data {
///     #[nullable]
///     a: MyOption<String>, // Error: `#[nullable]` may only be applied to fields of type `Option<T>`
/// }
/// ```
///
/// Likewise, if you define a type and name it `Option`, the `#[serde(...)]` attributes will
/// be added as if for an `Option<T>` field, but may silently behave incorrectly or raise a compile error.
///
/// ```compile_fail
/// # use serde::Serialize;
/// # use serde_option_macros::serde_option;
/// use std::vec::Vec as Option;
///
/// #[serde_option]
/// #[derive(Serialize)]
/// struct Data {
///     #[not_required]
///     a: Option<String>, // bad!
/// }
/// ```
///
/// [`serde`]: https://docs.rs/serde
/// [`serde_with`]: https://docs.rs/serde_with
#[proc_macro_attribute]
pub fn serde_option(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let res = process_items(item).unwrap_or_else(|err| err.to_compile_error());
    proc_macro::TokenStream::from(res)
}

/// Applies the `#[nullable]` and `#[not_required]` transformations on a field. This will only
/// work for fields whose type is statically assumed to be `Option<T>`
fn process_optional_field(field: &mut Field) -> Result<(), String> {
    // Detect and remove `#[nullable]` and `#[not_required]` attributes from the attribute list
    let mut nullable = false;
    let mut not_required = false;
    field.attrs.retain(|attr| {
        if attr.path().is_ident("nullable") {
            nullable = true;
            false
        } else if attr.path().is_ident("not_required") {
            not_required = true;
            false
        } else {
            true
        }
    });
    // `inner_type` is unused when the `"utoipa"` feature is disabled
    #[allow(unused_variables)]
    if let Some(inner_type) = get_std_option(&field.ty) {
        // Detect `#[serde(skip)]` and `#[serde(default)]` attributes
        let skipped = field_has_attribute(field, "serde", "skip");
        let default = field_has_attribute(field, "serde", "default");

        // The attributes are invalid and make no sense when combined with `#[serde(skip)]`
        if skipped && nullable {
            return Err("`#[nullable]` cannot be used in combination with `#[serde(skip)]`".into());
        } else if skipped && not_required {
            return Err(
                "`#[not_required]` cannot be used in combination with `#[serde(skip)]`".into(),
            );
        } else if default && not_required {
            return Err(
                "`#[not_required]` cannot be used in combination with `#[serde(default)]`".into(),
            );
        // Emit the appropriate serde attributes in the following cases
        } else if !nullable && not_required {
            field.attrs.push(parse_quote! {
                #[serde(default, skip_serializing_if = "Option::is_none",
                    with = "serde_with::rust::unwrap_or_skip")]
            });
            #[cfg(feature = "utoipa")]
            {
                field.attrs.push(parse_quote! {
                    #[schema(nullable = false)]
                })
            }
        } else if nullable && !not_required {
            field.attrs.push(parse_quote! {
                #[serde(with = "Option")]
            });
            #[cfg(feature = "utoipa")]
            {
                field.attrs.push(parse_quote! {
                    #[schema(required = true)]
                })
            }
        } else if nullable && not_required {
            field.attrs.push(parse_quote! {
                #[serde(default, skip_serializing_if = "Option::is_none",
                with = "serde_with::rust::double_option")]
            });
        }
    } else {
        // Error on use of `#[nullable]` or `#[not_required]` on non-Option fields
        if nullable {
            return Err("`#[nullable]` may only be used on fields of type `Option<T>`.".into());
        }
        if not_required {
            return Err("`#[not_required]` may only be used on fields of type `Option<T>`.".into());
        }
    }
    Ok(())
}

/// Determine if the `field` has an attribute with given `namespace` and `name`
///
/// On the example of `#[serde(default = "example")]`, `serde` is the namespace and `default` is the name.
fn field_has_attribute(field: &Field, namespace: &str, name: &str) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident(namespace) {
            let mut attribute_found = false;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(name) {
                    attribute_found = true;
                }
                Ok(())
            })
            .unwrap_or(());
            return attribute_found;
        }
    }
    false
}

/// Returns the type `T` whenever the type path refers to `std::option::Option<T>`.
/// Returns `None` otherwise.
///
/// # Accepts
///
/// * `Option<T>`
/// * `std::option::Option<T>`, with or without leading `::`
/// * `core::option::Option<T>`, with or without leading `::`
fn get_std_option(type_: &Type) -> Option<Type> {
    match type_ {
        // These syntax elements are all wrappers around types, so we recurse into them
        Type::Group(TypeGroup { elem, .. })
        | Type::Paren(TypeParen { elem, .. })
        | Type::Path(TypePath {
            qself: Some(QSelf { ty: elem, .. }),
            ..
        }) => get_std_option(elem),

        Type::Path(TypePath { qself: None, path }) => {
            let generic_args = if path.segments.len() == 1
                && path.leading_colon.is_none()
                && path.segments[0].ident == "Option"
            {
                Some(&path.segments[0].arguments)
            } else if path.segments.len() == 3
                && (path.segments[0].ident == "std" || path.segments[0].ident == "core")
                && path.segments[1].ident == "option"
                && path.segments[2].ident == "Option"
            {
                Some(&path.segments[2].arguments)
            } else {
                None
            };
            // Extract the `T` from `Option<T>` if possible.
            if let Some(PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                args,
                ..
            })) = generic_args
            {
                if args.len() == 1 {
                    if let GenericArgument::Type(ty) = &args[0] {
                        return Some(ty.clone());
                    }
                }
            };
            None
        }
        _ => None,
    }
}

/// Merge multiple [`syn::Error`] into one.
trait IteratorExt {
    fn merge_errors(self) -> Result<(), Error>
    where
        Self: Iterator<Item = Result<(), Error>> + Sized,
    {
        let accu = Ok(());
        self.fold(accu, |accu, error| match (accu, error) {
            (Ok(()), error) => error,
            (accu, Ok(())) => accu,
            (Err(mut err), Err(error)) => {
                err.combine(error);
                Err(err)
            }
        })
    }
}
impl<I> IteratorExt for I where I: Iterator<Item = Result<(), Error>> + Sized {}

/// Handle a single struct or a single enum variant
fn process_fields(fields: &mut Fields) -> Result<(), Error> {
    match fields {
        // simple, no fields, do nothing
        Fields::Unit => Ok(()),
        Fields::Named(ref mut fields) => fields
            .named
            .iter_mut()
            .map(|field| process_optional_field(field).map_err(|err| Error::new(field.span(), err)))
            .merge_errors(),
        Fields::Unnamed(ref mut fields) => fields
            .unnamed
            .iter_mut()
            .map(|field| process_optional_field(field).map_err(|err| Error::new(field.span(), err)))
            .merge_errors(),
    }
}

/// Apply function on every field of structs or enums
fn process_items(input: proc_macro::TokenStream) -> Result<proc_macro2::TokenStream, Error> {
    // Process the top level fields in structs
    if let Ok(mut input) = syn::parse::<ItemStruct>(input.clone()) {
        process_fields(&mut input.fields)?;
        Ok(quote!(#input))
    // Process the fields inside enum variants
    } else if let Ok(mut input) = syn::parse::<ItemEnum>(input) {
        input
            .variants
            .iter_mut()
            .map(|variant| process_fields(&mut variant.fields))
            .merge_errors()?;
        Ok(quote!(#input))
    } else {
        Err(Error::new(
            Span::call_site(),
            "The attribute can only be applied to struct or enum definitions.",
        ))
    }
}
