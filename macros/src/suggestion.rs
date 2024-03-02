use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Error};

use crate::util::{ApplyMeta, AttrArg};

pub fn impl_suggestion(ast: &syn::DeriveInput) -> Result<TokenStream, Error> {
    let syn::Data::Enum(ast_enum) = &ast.data else {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "YoetzSuggestion can only be derived from an enum",
        ));
    };
    let enum_data = SuggestionEnumData {
        visibility: ast.vis.clone(),
        name: ast.ident.clone(),
    };
    let variants_data = ast_enum
        .variants
        .iter()
        .map(|variant| SuggestionVariantData::new(&enum_data, variant))
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = TokenStream::default();
    for variant in variants_data.iter() {
        output.extend(variant.emit_strategy_code()?);
    }
    Ok(output)
}

#[derive(Debug)]
struct SuggestionEnumData {
    visibility: syn::Visibility,
    name: syn::Ident,
}

#[derive(Debug)]
struct SuggestionVariantData<'a> {
    parent: &'a SuggestionEnumData,
    strategy_name: syn::Ident,
    fields: syn::Fields,
    fields_config: Vec<FieldConfig>,
}

impl<'a> SuggestionVariantData<'a> {
    fn new(parent: &'a SuggestionEnumData, variant: &syn::Variant) -> Result<Self, Error> {
        let mut fields = variant.fields.clone();
        let fields_config = fields
            .iter_mut()
            .map(FieldConfig::new_for)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            parent,
            strategy_name: syn::Ident::new(
                &format!("{}{}", parent.name, variant.ident,),
                variant.ident.span(),
            ),
            fields,
            fields_config,
        })
    }

    fn semicolon_if_needed(&self) -> Option<syn::token::Semi> {
        if matches!(self.fields, syn::Fields::Named(..)) {
            None
        } else {
            Some(Default::default())
        }
    }

    fn emit_strategy_code(&self) -> Result<TokenStream, Error> {
        let strategy_name = &self.strategy_name;
        let mut fields = self.fields.clone();
        for (field, config) in fields.iter_mut().zip(self.fields_config.iter()) {
            field.vis = syn::Visibility::Public(Default::default());
            if config.role.unwrap() == FieldRole::Key {
                field.attrs.push(parse_quote!(#[allow(dead_code)]))
            }
        }
        let visibility = &self.parent.visibility;
        let semicolon = self.semicolon_if_needed();
        Ok(quote! {
            #[derive(bevy::ecs::component::Component)]
            #visibility struct #strategy_name #fields #semicolon
        })
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum FieldRole {
    Key,
    Input,
    State,
}

#[derive(Default, Debug)]
struct FieldConfig {
    role: Option<FieldRole>,
}

impl ApplyMeta for FieldConfig {
    fn apply_meta(&mut self, expr: AttrArg) -> Result<(), Error> {
        match expr.name().to_string().as_str() {
            role @ ("key" | "input" | "state") => match expr {
                AttrArg::Flag(_) => {
                    if self.role.is_some() {
                        return Err(Error::new_spanned(&expr, "field role given more than once"));
                    }
                    self.role = Some(match role {
                        "key" => FieldRole::Key,
                        "input" => FieldRole::Input,
                        "state" => FieldRole::State,
                        _ => panic!("Already filtered for one of these three"),
                    });
                    Ok(())
                }
                _ => Err(expr.incorrect_type()),
            },
            _ => Err(expr.unknown_name()),
        }
    }
}

impl FieldConfig {
    fn new_for(field: &mut syn::Field) -> Result<Self, Error> {
        let mut result = Self::default();
        for attr in field.attrs.drain(..) {
            if !attr.path().is_ident("yoetz") {
                continue;
            }
            result.apply_attr(&attr)?;
        }

        if result.role.is_none() {
            return Err(Error::new_spanned(&field, "YoetzSuggestion variant fields must be `#[yoets(<role>)]`, where <role> is key, input or state"));
        }

        Ok(result)
    }
}
