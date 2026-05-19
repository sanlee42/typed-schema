mod attr;
mod case;

use attr::{OpAttrs, SerdeContainer, SerdeField, ShapeAttrs};
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Field, Fields, GenericParam, Generics, Ident, Result, TypeParamBound,
    parse_macro_input, parse_quote,
};

#[proc_macro_derive(Shape, attributes(serde, shape))]
pub fn derive_shape(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_shape(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Op, attributes(op))]
pub fn derive_op(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_op(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_shape(input: DeriveInput) -> Result<TokenStream2> {
    let schema = schema_crate()?;
    let ident = input.ident;
    let attrs = ShapeAttrs::container(&input.attrs)?;
    let shape_name = attrs
        .name
        .ok_or_else(|| Error::new_spanned(&ident, "missing #[shape(name = \"...\")]"))?;
    let desc = opt_string(&attrs.desc);
    let serde = SerdeContainer::from_attrs(&input.attrs)?;
    let fields = fields(&input.data, &serde, &schema)?;
    let generics = add_json_schema_bounds(input.generics, &schema);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #schema::Shape for #ident #ty_generics #where_clause {
            fn shape() -> #schema::Type {
                #schema::Type {
                    name: ::std::string::String::from(#shape_name),
                    rust: ::std::format!("{}::{}", ::std::module_path!(), ::std::stringify!(#ident)),
                    desc: #desc,
                    schema: #schema::json_schema::<Self>(),
                    fields: ::std::vec![#(#fields),*],
                }
            }

            fn name() -> &'static str {
                #shape_name
            }
        }
    })
}

fn expand_op(input: DeriveInput) -> Result<TokenStream2> {
    let schema = schema_crate()?;
    let ident = input.ident;
    if !input.generics.params.is_empty() {
        return Err(Error::new_spanned(
            input.generics,
            "Op marker structs cannot be generic",
        ));
    }
    match input.data {
        Data::Struct(data) if matches!(data.fields, Fields::Unit) => {}
        _ => {
            return Err(Error::new_spanned(
                &ident,
                "Op can only be derived for unit marker structs",
            ));
        }
    }

    let attrs = OpAttrs::from_attrs(&input.attrs)?;
    let name = attrs
        .name
        .ok_or_else(|| Error::new_spanned(&ident, "missing op name"))?;
    let input_ty = attrs
        .input
        .ok_or_else(|| Error::new_spanned(&ident, "missing op input"))?;
    let output_ty = attrs
        .output
        .ok_or_else(|| Error::new_spanned(&ident, "missing op output"))?;
    let desc = opt_string(&attrs.desc);
    let http = match attrs.http {
        Some(http) => {
            let method = method(&http.method, &schema)?;
            let path = http.path;
            quote! {
                ::std::option::Option::Some(#schema::Http {
                    method: #method,
                    path: ::std::string::String::from(#path),
                })
            }
        }
        None => quote! { ::std::option::Option::None },
    };

    Ok(quote! {
        impl #schema::OpSpec for #ident {
            fn op() -> #schema::Op {
                #schema::Op {
                    name: ::std::string::String::from(#name),
                    desc: #desc,
                    input: <#input_ty as #schema::Shape>::name().to_owned(),
                    output: <#output_ty as #schema::Shape>::name().to_owned(),
                    http: #http,
                }
            }
        }
    })
}

fn fields(
    data: &Data,
    container: &SerdeContainer,
    schema: &TokenStream2,
) -> Result<Vec<TokenStream2>> {
    match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|field| match named_field(field, container, schema) {
                    Ok(Some(field)) => Some(Ok(field)),
                    Ok(None) => None,
                    Err(err) => Some(Err(err)),
                })
                .collect::<Result<Vec<_>>>(),
            Fields::Unnamed(fields) => {
                for field in &fields.unnamed {
                    reject_field_shape_attrs(field)?;
                }
                Ok(Vec::new())
            }
            Fields::Unit => Ok(Vec::new()),
        },
        Data::Enum(data) => {
            for variant in &data.variants {
                for attr in &variant.attrs {
                    if attr.path().is_ident("shape") {
                        return Err(Error::new_spanned(
                            attr,
                            "shape attributes on enum variants are not supported",
                        ));
                    }
                }
                for field in &variant.fields {
                    reject_field_shape_attrs(field)?;
                }
            }
            Ok(Vec::new())
        }
        Data::Union(data) => Err(Error::new_spanned(
            data.union_token,
            "Shape cannot be derived for unions",
        )),
    }
}

fn named_field(
    field: &Field,
    container: &SerdeContainer,
    schema: &TokenStream2,
) -> Result<Option<TokenStream2>> {
    let ident = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "expected named field"))?;
    let shape = ShapeAttrs::field(&field.attrs)?;
    let serde = SerdeField::from_attrs(&field.attrs)?;
    if serde.skip || serde.skip_serializing || serde.flatten {
        if shape.any() {
            return Err(Error::new_spanned(
                field,
                "shape attributes cannot be used on skipped or flattened serde fields",
            ));
        }
        return Ok(None);
    }

    let ident_name = ident.to_string();
    let field_name = field_name(&ident_name, &shape, &serde, container)?;
    let field_ident = ident_name;
    let field_ty = &field.ty;
    let metric = match shape.metric(field)? {
        Some(metric) => {
            let calc = metric.calc;
            let grain = opt_expr_string(&metric.grain);
            let sources = metric.sources.into_iter().map(|source| {
                quote! { ::std::string::String::from(#source) }
            });
            quote! {
                ::std::option::Option::Some(#schema::Metric {
                    calc: ::std::string::String::from(#calc),
                    grain: #grain,
                    sources: ::std::vec![#(#sources),*],
                })
            }
        }
        None => quote! { ::std::option::Option::None },
    };

    Ok(Some(quote! {
        #schema::Field {
            name: ::std::string::String::from(#field_name),
            ident: ::std::string::String::from(#field_ident),
            ty: ::std::string::String::from(::std::stringify!(#field_ty)),
            metric: #metric,
        }
    }))
}

fn field_name(
    ident_name: &str,
    shape: &ShapeAttrs,
    serde: &SerdeField,
    container: &SerdeContainer,
) -> Result<String> {
    if let Some(name) = &shape.name {
        return Ok(name.value());
    }
    if let Some(rename) = &serde.rename {
        return Ok(rename.value());
    }
    if let Some(rename) = &serde.directional_rename {
        return Err(Error::new_spanned(
            rename,
            "directional serde rename requires #[shape(name = \"...\")]",
        ));
    }
    if let Some(rename_all) = &container.directional_rename_all {
        return Err(Error::new_spanned(
            rename_all,
            "directional serde rename_all requires field-level #[shape(name = \"...\")]",
        ));
    }
    if let Some(rename_all) = &container.rename_all {
        return case::rename_all(&rename_all.value(), ident_name).ok_or_else(|| {
            Error::new_spanned(
                rename_all,
                "unsupported serde rename_all; expected lowercase, UPPERCASE, PascalCase, camelCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case, or SCREAMING-KEBAB-CASE",
            )
        });
    }
    Ok(ident_name.to_owned())
}

fn reject_field_shape_attrs(field: &Field) -> Result<()> {
    for attr in &field.attrs {
        if attr.path().is_ident("shape") {
            return Err(Error::new_spanned(
                attr,
                "shape field attributes are only supported on named struct fields",
            ));
        }
    }
    Ok(())
}

fn add_json_schema_bounds(mut generics: Generics, schema: &TokenStream2) -> Generics {
    let bound: TypeParamBound = parse_quote!(#schema::schemars::JsonSchema);
    for param in &mut generics.params {
        if let GenericParam::Type(ty) = param {
            ty.bounds.push(bound.clone());
        }
    }
    generics
}

fn method(method: &Ident, schema: &TokenStream2) -> Result<TokenStream2> {
    match method.to_string().to_ascii_uppercase().as_str() {
        "GET" => Ok(quote! { #schema::Method::Get }),
        "POST" => Ok(quote! { #schema::Method::Post }),
        "PUT" => Ok(quote! { #schema::Method::Put }),
        "PATCH" => Ok(quote! { #schema::Method::Patch }),
        "DELETE" => Ok(quote! { #schema::Method::Delete }),
        _ => Err(Error::new_spanned(
            method,
            "unsupported HTTP method; expected GET, POST, PUT, PATCH, or DELETE",
        )),
    }
}

fn opt_string(value: &Option<syn::LitStr>) -> TokenStream2 {
    match value {
        Some(value) => quote! { ::std::option::Option::Some(::std::string::String::from(#value)) },
        None => quote! { ::std::option::Option::None },
    }
}

fn opt_expr_string(value: &Option<syn::Expr>) -> TokenStream2 {
    match value {
        Some(value) => quote! { ::std::option::Option::Some(::std::string::String::from(#value)) },
        None => quote! { ::std::option::Option::None },
    }
}

fn schema_crate() -> Result<TokenStream2> {
    match crate_name("typed-schema") {
        Ok(FoundCrate::Itself) => Ok(quote! { ::typed_schema }),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            Ok(quote! { ::#ident })
        }
        Err(err) => Err(Error::new(Span::call_site(), err)),
    }
}
