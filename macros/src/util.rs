use std::iter;

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Error, Token,
};

pub enum AttrArg {
    Flag(Ident),
    KeyValue(KeyValue),
    Sub(SubAttr),
    Not { not: Token![!], name: Ident },
}

impl AttrArg {
    pub fn name(&self) -> &Ident {
        match self {
            AttrArg::Flag(name) => name,
            AttrArg::KeyValue(KeyValue { name, .. }) => name,
            AttrArg::Sub(SubAttr { name, .. }) => name,
            AttrArg::Not { name, .. } => name,
        }
    }

    pub fn incorrect_type(&self) -> syn::Error {
        let message = match self {
            AttrArg::Flag(name) => format!("{:?} is not supported as a flag", name.to_string()),
            AttrArg::KeyValue(KeyValue { name, .. }) => {
                format!("{:?} is not supported as key-value", name.to_string())
            }
            AttrArg::Sub(SubAttr { name, .. }) => format!(
                "{:?} is not supported as nested attribute",
                name.to_string()
            ),
            AttrArg::Not { name, .. } => format!("{:?} cannot be nullified", name.to_string()),
        };
        syn::Error::new_spanned(self, message)
    }

    pub fn unknown_name(&self) -> syn::Error {
        Error::new_spanned(
            self.name(),
            format!("Unknown parameter {:?}", self.name().to_string()),
        )
    }

    #[allow(dead_code)]
    pub fn flag(self) -> syn::Result<Ident> {
        if let Self::Flag(name) = self {
            Ok(name)
        } else {
            Err(self.incorrect_type())
        }
    }

    #[allow(dead_code)]
    pub fn key_value(self) -> syn::Result<KeyValue> {
        if let Self::KeyValue(key_value) = self {
            Ok(key_value)
        } else {
            Err(self.incorrect_type())
        }
    }

    #[allow(dead_code)]
    pub fn key_value_or_not(self) -> syn::Result<Option<KeyValue>> {
        match self {
            Self::KeyValue(key_value) => Ok(Some(key_value)),
            Self::Not { .. } => Ok(None),
            _ => Err(self.incorrect_type()),
        }
    }

    pub fn sub_attr(self) -> syn::Result<SubAttr> {
        if let Self::Sub(sub_attr) = self {
            Ok(sub_attr)
        } else {
            Err(self.incorrect_type())
        }
    }

    #[allow(dead_code)]
    pub fn apply_flag_to_field(self, field: &mut Option<Span>, caption: &str) -> syn::Result<()> {
        match self {
            AttrArg::Flag(flag) => {
                if field.is_none() {
                    *field = Some(flag.span());
                    Ok(())
                } else {
                    Err(Error::new(
                        flag.span(),
                        format!("Illegal setting - field is already {caption}"),
                    ))
                }
            }
            AttrArg::Not { .. } => {
                *field = None;
                Ok(())
            }
            _ => Err(self.incorrect_type()),
        }
    }
}

pub struct KeyValue {
    pub name: Ident,
    pub eq: Token![=],
    pub value: TokenStream,
}

impl KeyValue {
    #[allow(dead_code)]
    pub fn parse_value<T: Parse>(self) -> syn::Result<T> {
        syn::parse2(self.value)
    }
}

impl ToTokens for KeyValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.value.to_tokens(tokens);
    }
}

impl Parse for KeyValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

pub struct SubAttr {
    pub name: Ident,
    pub paren: token::Paren,
    pub args: TokenStream,
}

impl SubAttr {
    pub fn args<T: Parse>(self) -> syn::Result<impl IntoIterator<Item = T>> {
        Punctuated::<T, Token![,]>::parse_terminated.parse2(self.args)
    }

    #[allow(dead_code)]
    pub fn undelimited<T: Parse>(self) -> syn::Result<impl IntoIterator<Item = T>> {
        (|p: ParseStream| {
            iter::from_fn(|| (!p.is_empty()).then(|| p.parse())).collect::<syn::Result<Vec<T>>>()
        })
        .parse2(self.args)
    }
}

impl ToTokens for SubAttr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.paren.surround(tokens, |t| self.args.to_tokens(t));
    }
}

fn get_cursor_after_parsing<P: Parse + Spanned>(
    input: syn::parse::ParseBuffer,
) -> syn::Result<syn::buffer::Cursor> {
    let parse_attempt: P = input.parse()?;
    let cursor = input.cursor();
    if cursor.eof() || input.peek(Token![,]) {
        Ok(cursor)
    } else {
        Err(syn::Error::new(
            parse_attempt.span(),
            "does not end with comma or end of section",
        ))
    }
}

fn get_token_stream_up_to_cursor(
    input: syn::parse::ParseStream,
    cursor: syn::buffer::Cursor,
) -> syn::Result<TokenStream> {
    Ok(core::iter::from_fn(|| {
        if input.cursor() < cursor {
            input.parse::<TokenTree>().ok()
        } else {
            None
        }
    })
    .collect())
}

impl Parse for AttrArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![!]) {
            Ok(Self::Not {
                not: input.parse()?,
                name: input.parse()?,
            })
        } else {
            let name = input.parse()?;
            if input.peek(Token![,]) || input.is_empty() {
                Ok(Self::Flag(name))
            } else if input.peek(token::Paren) {
                let args;
                Ok(Self::Sub(SubAttr {
                    name,
                    paren: parenthesized!(args in input),
                    args: args.parse()?,
                }))
            } else if input.peek(Token![=]) {
                // Try parsing as a type first, because it _should_ be simpler

                Ok(Self::KeyValue(KeyValue {
                    name,
                    eq: input.parse()?,
                    value: {
                        let cursor = get_cursor_after_parsing::<syn::Type>(input.fork())
                            .or_else(|_| get_cursor_after_parsing::<syn::Expr>(input.fork()))?;
                        get_token_stream_up_to_cursor(input, cursor)?
                    },
                }))
            } else {
                Err(input.error("expected !<ident>, <ident>=<value> or <ident>(…)"))
            }
        }
    }
}

impl ToTokens for AttrArg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            AttrArg::Flag(flag) => flag.to_tokens(tokens),
            AttrArg::KeyValue(kv) => kv.to_tokens(tokens),
            AttrArg::Sub(sub) => sub.to_tokens(tokens),
            AttrArg::Not { not, name } => {
                not.to_tokens(tokens);
                name.to_tokens(tokens);
            }
        }
    }
}

pub trait ApplyMeta {
    fn apply_meta(&mut self, expr: AttrArg) -> Result<(), Error>;

    fn apply_sub_attr(&mut self, sub_attr: SubAttr) -> syn::Result<()> {
        for arg in sub_attr.args()? {
            self.apply_meta(arg)?;
        }
        Ok(())
    }

    fn apply_subsections(&mut self, list: &syn::MetaList) -> syn::Result<()> {
        if list.tokens.is_empty() {
            return Err(syn::Error::new_spanned(list, "Expected builder(…)"));
        }

        let parser = syn::punctuated::Punctuated::<_, syn::token::Comma>::parse_terminated;
        let exprs = parser.parse2(list.tokens.clone())?;
        for expr in exprs {
            self.apply_meta(expr)?;
        }

        Ok(())
    }

    fn apply_attr(&mut self, attr: &Attribute) -> syn::Result<()> {
        match &attr.meta {
            syn::Meta::List(list) => self.apply_subsections(list),
            meta => Err(Error::new_spanned(meta, "Expected builder(…)")),
        }
    }
}
