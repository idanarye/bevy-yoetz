use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, Error};

use crate::util::{ApplyMeta, AttrArg};

pub fn impl_suggestion(ast: &syn::DeriveInput) -> Result<TokenStream, Error> {
    let syn::Data::Enum(ast_enum) = &ast.data else {
        return Err(Error::new(
            Span::call_site(),
            "YoetzSuggestion can only be derived from an enum",
        ));
    };
    let enum_data = SuggestionEnumData {
        visibility: ast.vis.clone(),
        name: ast.ident.clone(),
        key_enum_name: syn::Ident::new(&format!("{}Key", ast.ident), ast.ident.span()),
        omni_query_name: syn::Ident::new(&format!("{}OmniQuery", ast.ident), ast.ident.span()),
    };
    let variants_data = ast_enum
        .variants
        .iter()
        .map(|variant| SuggestionVariantData::new(&enum_data, variant))
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = TokenStream::default();

    output.extend(enum_data.emit_key_enum_code(&variants_data)?);
    output.extend(enum_data.emit_omni_query_code(&variants_data)?);

    for variant in variants_data.iter() {
        output.extend(variant.emit_strategy_code()?);
    }

    Ok(output)
}

struct SuggestionEnumData {
    visibility: syn::Visibility,
    name: syn::Ident,
    key_enum_name: syn::Ident,
    omni_query_name: syn::Ident,
}

impl SuggestionEnumData {
    fn emit_key_enum_code(&self, variants: &[SuggestionVariantData]) -> Result<TokenStream, Error> {
        let visibility = &self.visibility;
        let key_enum_name = &self.key_enum_name;
        let variant_options = variants
            .iter()
            .map(|variant| variant.emit_key_enum_variant())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(quote! {
            #[derive(Clone, PartialEq)]
            #visibility enum #key_enum_name {
                #(#variant_options,)*
            }
        })
    }

    fn emit_omni_query_code(
        &self,
        variants: &[SuggestionVariantData],
    ) -> Result<TokenStream, Error> {
        let omni_query_name = &self.omni_query_name;
        let strategies = variants.iter().enumerate().map(|(i, variant)| {
            let field_name = syn::Ident::new(&format!("strategy{i}"), Span::call_site());
            let component_type = &variant.strategy_name;
            quote!(
                #field_name: Option<&'static mut #component_type>
            )
        });
        Ok(quote! {
            #[derive(bevy::ecs::query::QueryData)]
            #[query_data(mutable)]
            struct #omni_query_name {
                #(#strategies,)*
            }
        })
    }
}

struct SuggestionVariantData<'a> {
    parent: &'a SuggestionEnumData,
    name: syn::Ident,
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

    fn emit_strategy_code(&self) -> Result<TokenStream, Error> {
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
        Ok(quote! {
            #[derive(bevy::ecs::component::Component)]
            #visibility struct #strategy_name #fields #semicolon
        })
    }

    fn emit_key_enum_variant(&self) -> Result<TokenStream, Error> {
        let name = &self.name;
        let fields = self
            .fields
            .iter()
            .zip(&self.fields_config)
            .filter_map(|(field, config)| {
                if config.role.unwrap() == FieldRole::Key {
                    Some(field.clone())
                } else {
                    None
                }
            })
            .collect();
        let fields = match &self.fields {
            syn::Fields::Named(named) => syn::Fields::Named(syn::FieldsNamed {
                brace_token: named.brace_token,
                named: fields,
            }),
            syn::Fields::Unnamed(unnamed) => syn::Fields::Unnamed(syn::FieldsUnnamed {
                paren_token: unnamed.paren_token,
                unnamed: fields,
            }),
            syn::Fields::Unit => syn::Fields::Unit,
        };
        Ok(quote! {
            #name #fields
        })
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum FieldRole {
    Key,
    Input,
    State,
}

#[derive(Default)]
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
