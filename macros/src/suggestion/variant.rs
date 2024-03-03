use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Error};

use super::field::{FieldConfig, FieldRole};
use super::suggestion_enum::SuggestionEnumData;

pub struct SuggestionVariantData<'a> {
    pub parent: &'a SuggestionEnumData,
    pub name: syn::Ident,
    pub strategy_name: syn::Ident,
    pub fields: syn::Fields,
    pub fields_config: Vec<FieldConfig>,
}

impl<'a> SuggestionVariantData<'a> {
    pub fn new(parent: &'a SuggestionEnumData, variant: &syn::Variant) -> Result<Self, Error> {
        let mut fields = variant.fields.clone();
        let fields_config = fields
            .iter_mut()
            .map(FieldConfig::new_for)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            parent,
            name: variant.ident.clone(),
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

    pub fn emit_strategy_code(&self) -> Result<TokenStream, Error> {
        let strategy_name = &self.strategy_name;
        let mut fields = self.fields.clone();
        for (field, config) in fields.iter_mut().zip(self.fields_config.iter()) {
            field.vis = self.parent.visibility.clone();
            if config.role.unwrap() == FieldRole::Key {
                field.attrs.push(parse_quote!(#[allow(dead_code)]))
            }
        }
        let visibility = &self.parent.visibility;
        let semicolon = self.semicolon_if_needed();
        let extra_derives = &self.parent.strategy_structs_config.derive;
        Ok(quote! {
            #[derive(bevy::ecs::component::Component, #(#extra_derives),*)]
            #visibility struct #strategy_name #fields #semicolon
        })
    }

    pub fn iter_fields_with_configs(&self) -> impl Iterator<Item = (&syn::Field, &FieldConfig)> {
        self.fields.iter().zip(&self.fields_config)
    }

    pub fn iter_key_fields(&self) -> impl Iterator<Item = &syn::Field> {
        self.iter_fields_with_configs()
            .filter_map(|(field, config)| {
                if config.role.unwrap() == FieldRole::Key {
                    Some(field)
                } else {
                    None
                }
            })
    }

    pub fn emit_key_enum_variant(&self) -> Result<TokenStream, Error> {
        let name = &self.name;
        let fields = match &self.fields {
            syn::Fields::Named(named) => syn::Fields::Named(syn::FieldsNamed {
                brace_token: named.brace_token,
                named: self.iter_key_fields().cloned().collect(),
            }),
            syn::Fields::Unnamed(unnamed) => {
                return Err(Error::new_spanned(
                    unnamed,
                    "tuple variants are currently unsupported for YoetzSuggestion, \
                    and are resuseved for future features",
                ));
            }
            syn::Fields::Unit => syn::Fields::Unit,
        };
        Ok(quote! {
            #name #fields
        })
    }
}
