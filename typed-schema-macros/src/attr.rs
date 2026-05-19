use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Error, Expr, Ident, LitStr, Result, Token, Type, parenthesized};

#[derive(Default)]
pub struct ShapeAttrs {
    pub name: Option<LitStr>,
    pub desc: Option<LitStr>,
    pub calc: Option<LitStr>,
    pub grain: Option<Expr>,
    pub sources: Vec<Expr>,
}

impl ShapeAttrs {
    pub fn container(attrs: &[Attribute]) -> Result<Self> {
        let mut parsed = Self::default();
        parsed.parse(attrs, ShapeAttrTarget::Container)?;
        Ok(parsed)
    }

    pub fn field(attrs: &[Attribute]) -> Result<Self> {
        let mut parsed = Self::default();
        parsed.parse(attrs, ShapeAttrTarget::Field)?;
        Ok(parsed)
    }

    pub fn any(&self) -> bool {
        self.name.is_some()
            || self.desc.is_some()
            || self.calc.is_some()
            || self.grain.is_some()
            || !self.sources.is_empty()
    }

    pub fn metric<T: ToTokens>(&self, target: T) -> Result<Option<MetricAttrs>> {
        let any = self.calc.is_some() || self.grain.is_some() || !self.sources.is_empty();
        if !any {
            return Ok(None);
        }

        Ok(Some(MetricAttrs {
            calc: required(&self.calc, &target, "calc")?,
            grain: self.grain.clone(),
            sources: self.sources.clone(),
        }))
    }

    fn parse(&mut self, attrs: &[Attribute], target: ShapeAttrTarget) -> Result<()> {
        for attr in attrs {
            if !attr.path().is_ident("shape") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?.parse()?;
                    set_once(&mut self.name, value, "duplicate shape name")?;
                    return Ok(());
                }
                if meta.path.is_ident("desc") {
                    if target != ShapeAttrTarget::Container {
                        return Err(meta.error("shape desc is only supported on containers"));
                    }
                    let value = meta.value()?.parse()?;
                    set_once(&mut self.desc, value, "duplicate shape desc")?;
                    return Ok(());
                }
                if meta.path.is_ident("calc") {
                    if target != ShapeAttrTarget::Field {
                        return Err(meta.error("shape calc is only supported on fields"));
                    }
                    let value = meta.value()?.parse()?;
                    set_once(&mut self.calc, value, "duplicate shape calc")?;
                    return Ok(());
                }
                if meta.path.is_ident("grain") {
                    if target != ShapeAttrTarget::Field {
                        return Err(meta.error("shape grain is only supported on fields"));
                    }
                    let value = meta.value()?.parse()?;
                    set_once(&mut self.grain, value, "duplicate shape grain")?;
                    return Ok(());
                }
                if meta.path.is_ident("source") {
                    if target != ShapeAttrTarget::Field {
                        return Err(meta.error("shape source is only supported on fields"));
                    }
                    let value = meta.value()?.parse()?;
                    self.sources.push(value);
                    return Ok(());
                }
                Err(meta.error("unsupported shape attribute"))
            })?;
        }
        Ok(())
    }
}

pub struct MetricAttrs {
    pub calc: LitStr,
    pub grain: Option<Expr>,
    pub sources: Vec<Expr>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeAttrTarget {
    Container,
    Field,
}

#[derive(Default)]
pub struct SerdeContainer {
    pub rename_all: Option<LitStr>,
    pub directional_rename_all: Option<LitStr>,
}

impl SerdeContainer {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut out = Self::default();
        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename_all") {
                    if meta.input.peek(Token![=]) {
                        let value = meta.value()?.parse()?;
                        set_once(&mut out.rename_all, value, "duplicate serde rename_all")?;
                        return Ok(());
                    }
                    meta.parse_nested_meta(|nested| {
                        if nested.path.is_ident("serialize") || nested.path.is_ident("deserialize")
                        {
                            if out.directional_rename_all.is_none() {
                                out.directional_rename_all = Some(nested.value()?.parse()?);
                            } else {
                                let _ = nested.value()?.parse::<LitStr>()?;
                            }
                            return Ok(());
                        }
                        Ok(())
                    })?;
                    return Ok(());
                }
                consume_ignored_meta(meta)
            })?;
        }
        Ok(out)
    }
}

#[derive(Default)]
pub struct SerdeField {
    pub rename: Option<LitStr>,
    pub directional_rename: Option<LitStr>,
    pub skip: bool,
    pub skip_serializing: bool,
    pub flatten: bool,
}

impl SerdeField {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut out = Self::default();
        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    if meta.input.peek(Token![=]) {
                        let value = meta.value()?.parse()?;
                        set_once(&mut out.rename, value, "duplicate serde rename")?;
                        return Ok(());
                    }
                    meta.parse_nested_meta(|nested| {
                        if nested.path.is_ident("serialize") || nested.path.is_ident("deserialize")
                        {
                            if out.directional_rename.is_none() {
                                out.directional_rename = Some(nested.value()?.parse()?);
                            } else {
                                let _ = nested.value()?.parse::<LitStr>()?;
                            }
                            return Ok(());
                        }
                        Ok(())
                    })?;
                    return Ok(());
                }
                if meta.path.is_ident("skip") {
                    out.skip = true;
                    return Ok(());
                }
                if meta.path.is_ident("skip_serializing") {
                    out.skip_serializing = true;
                    return Ok(());
                }
                if meta.path.is_ident("flatten") {
                    out.flatten = true;
                    return Ok(());
                }
                consume_ignored_meta(meta)
            })?;
        }
        Ok(out)
    }
}

#[derive(Default)]
pub struct OpAttrs {
    pub name: Option<LitStr>,
    pub input: Option<Type>,
    pub output: Option<Type>,
    pub http: Option<HttpArg>,
    pub desc: Option<LitStr>,
}

impl OpAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut out = Self::default();
        let mut found = false;
        for attr in attrs {
            if !attr.path().is_ident("op") {
                continue;
            }
            if found {
                return Err(Error::new_spanned(attr, "duplicate op attribute"));
            }
            found = true;
            let args = attr.parse_args_with(Punctuated::<OpArg, Token![,]>::parse_terminated)?;
            for arg in args {
                match arg {
                    OpArg::Name(value) => set_once(&mut out.name, value, "duplicate op name")?,
                    OpArg::Input(value) => set_once(&mut out.input, value, "duplicate op input")?,
                    OpArg::Output(value) => {
                        set_once(&mut out.output, value, "duplicate op output")?
                    }
                    OpArg::Http(value) => {
                        if out.http.is_some() {
                            return Err(Error::new_spanned(value.method, "duplicate op http"));
                        }
                        out.http = Some(value);
                    }
                    OpArg::Desc(value) => set_once(&mut out.desc, value, "duplicate op desc")?,
                }
            }
        }
        if !found {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                "missing #[op(...)] attribute",
            ));
        }
        Ok(out)
    }
}

pub enum OpArg {
    Name(LitStr),
    Input(Type),
    Output(Type),
    Http(HttpArg),
    Desc(LitStr),
}

impl Parse for OpArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key: Ident = input.parse()?;
        if key == "http" {
            let content;
            parenthesized!(content in input);
            let method: Ident = content.parse()?;
            content.parse::<Token![,]>()?;
            let path: LitStr = content.parse()?;
            return Ok(Self::Http(HttpArg { method, path }));
        }

        input.parse::<Token![=]>()?;
        if key == "name" {
            return Ok(Self::Name(input.parse()?));
        }
        if key == "input" {
            return Ok(Self::Input(input.parse()?));
        }
        if key == "output" {
            return Ok(Self::Output(input.parse()?));
        }
        if key == "desc" {
            return Ok(Self::Desc(input.parse()?));
        }

        Err(Error::new_spanned(key, "unsupported op attribute"))
    }
}

pub struct HttpArg {
    pub method: Ident,
    pub path: LitStr,
}

fn set_once<T: ToTokens>(slot: &mut Option<T>, value: T, message: &str) -> Result<()> {
    if slot.is_some() {
        return Err(Error::new_spanned(value, message));
    }
    *slot = Some(value);
    Ok(())
}

fn consume_ignored_meta(meta: syn::meta::ParseNestedMeta<'_>) -> Result<()> {
    if meta.input.peek(Token![=]) {
        let _ = meta.value()?.parse::<Expr>()?;
    }
    Ok(())
}

fn required<T: Clone, U: ToTokens>(value: &Option<T>, target: U, name: &str) -> Result<T> {
    value.clone().ok_or_else(|| {
        Error::new_spanned(
            target,
            format!("metric shape requires {name} when any metric attribute is present"),
        )
    })
}
