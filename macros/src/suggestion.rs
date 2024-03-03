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
    output.extend(enum_data.emit_trait_impl(&variants_data)?);

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
            let strategy_field_name = syn::Ident::new(&format!("strategy{i}"), Span::call_site());
            let component_type = &variant.strategy_name;
            quote!(
                #strategy_field_name: Option<&'static mut #component_type>
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

    fn emit_trait_impl(&self, variants: &[SuggestionVariantData]) -> Result<TokenStream, Error> {
        let Self {
            visibility: _,
            name: suggestion_enum_name,
            key_enum_name,
            omni_query_name,
        } = self;
        let key_method = self.emit_key_method(variants)?;
        let remove_components_method = self.emit_remove_components_method(variants)?;
        let add_components_method = self.emit_add_components_method(variants)?;
        let update_into_components_method = self.emit_update_into_components_method(variants)?;
        Ok(quote! {
            impl YoetzSuggestion for #suggestion_enum_name {
                type Key = #key_enum_name;
                type OmniQuery = #omni_query_name;

                #key_method
                #remove_components_method
                #add_components_method
                #update_into_components_method
            }
        })
    }

    fn emit_key_method(&self, variants: &[SuggestionVariantData]) -> Result<TokenStream, Error> {
        let suggestion_enum_name = &self.name;
        let key_enum_name = &self.key_enum_name;

        let mut variants_code = TokenStream::default();

        for variant in variants {
            let variant_name = &variant.name;
            let (source_pattern, target_pattern) = match &variant.fields {
                syn::Fields::Named(_) => {
                    let get_fields = variant.iter_fields_with_configs().map(|(field, config)| {
                        let field_name = &field.ident;
                        if config.role.unwrap() == FieldRole::Key {
                            quote!(#field_name)
                        } else {
                            quote!(#field_name: _)
                        }
                    });
                    let set_fields = variant.iter_key_fields().map(|field| {
                        let field_name = &field.ident;
                        quote!(#field_name: #field_name.clone())
                    });
                    (
                        quote!({
                            #(
                                #get_fields
                            ),*
                        }),
                        quote!({
                            #(
                                #set_fields
                            ),*
                        }),
                    )
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => (quote!(), quote!()),
            };

            variants_code.extend(quote! {
                #suggestion_enum_name::#variant_name #source_pattern => #key_enum_name::#variant_name #target_pattern,
            });
        }

        Ok(quote! {
            fn key(&self) -> Self::Key {
                match self {
                    #variants_code
                }
            }
        })
    }

    fn emit_remove_components_method(
        &self,
        variants: &[SuggestionVariantData],
    ) -> Result<TokenStream, Error> {
        let key_enum_name = &self.key_enum_name;

        let mut variants_code = TokenStream::default();

        for variant in variants {
            let variant_name = &variant.name;
            let fields_pattern = match variant.fields {
                syn::Fields::Named(_) => quote!({ .. }),
                syn::Fields::Unnamed(_) => quote!((..)),
                syn::Fields::Unit => quote!(),
            };
            let strategy_name = &variant.strategy_name;
            variants_code.extend(quote! {
                #key_enum_name::#variant_name #fields_pattern => {
                    cmd.remove::<#strategy_name>();
                }
            })
        }

        Ok(quote! {
            fn remove_components(key: &Self::Key, cmd: &mut bevy::ecs::system::EntityCommands) {
                match key {
                    #variants_code
                }
            }
        })
    }

    fn emit_add_components_method(
        &self,
        variants: &[SuggestionVariantData],
    ) -> Result<TokenStream, Error> {
        let suggestion_enum_name = &self.name;

        let mut variants_code = TokenStream::default();

        for variant in variants {
            let variant_name = &variant.name;
            let strategy_name = &variant.strategy_name;
            let fields = variant
                .fields
                .iter()
                .map(|field| &field.ident)
                .collect::<Vec<_>>();

            variants_code.extend(match &variant.fields {
                syn::Fields::Named(_) => quote! {
                    #suggestion_enum_name::#variant_name { #(#fields),* } => {
                        cmd.insert(#strategy_name {
                            #(
                                #fields: #fields.clone()
                            ),*
                        });
                    }
                },
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => quote! {
                    #suggestion_enum_name::#variant_name => {
                        cmd.insert(#strategy_name);
                    }
                },
            });
        }

        Ok(quote! {
            fn add_components(&self, cmd: &mut bevy::ecs::system::EntityCommands) {
                match self {
                    #variants_code
                }
            }
        })
    }

    fn emit_update_into_components_method(
        &self,
        variants: &[SuggestionVariantData],
    ) -> Result<TokenStream, Error> {
        let suggestion_enum_name = &self.name;

        let mut variants_code = TokenStream::default();

        for (i, variant) in variants.iter().enumerate() {
            let strategy_field_name = syn::Ident::new(&format!("strategy{i}"), Span::call_site());
            let variant_name = &variant.name;
            // let strategy_name = &variant.strategy_name;

            let fields_pattern;
            let update_statements;
            match &variant.fields {
                syn::Fields::Named(named) => {
                    let all_fields = named.named.iter().map(|field| &field.ident);
                    fields_pattern = quote!({ #(#all_fields),* });

                    update_statements = variant
                        .iter_fields_with_configs()
                        .filter_map(|(field, config)| {
                            if config.role.unwrap() == FieldRole::Input {
                                let field_name = &field.ident;
                                Some(quote! {
                                    strategy_component.#field_name = #field_name;
                                })
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => {
                    fields_pattern = quote!();
                    update_statements = Vec::default();
                }
            }

            variants_code.extend(quote! {
                #suggestion_enum_name::#variant_name #fields_pattern => {
                    if let Some(strategy_component) = components.#strategy_field_name.as_mut() {
                        #( #update_statements )*
                        Ok(())
                    } else {
                        Err(#suggestion_enum_name::#variant_name #fields_pattern)
                    }
                }
            })
        }

        Ok(quote! {
            fn update_into_components(
                self,
                components: &mut <Self::OmniQuery as bevy::ecs::query::WorldQuery>::Item<'_>,
            ) -> Result<(), Self> {
                match self {
                    #variants_code
                }
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

    fn iter_fields_with_configs(&self) -> impl Iterator<Item = (&syn::Field, &FieldConfig)> {
        self.fields.iter().zip(&self.fields_config)
    }

    fn iter_key_fields(&self) -> impl Iterator<Item = &syn::Field> {
        self.iter_fields_with_configs()
            .filter_map(|(field, config)| {
                if config.role.unwrap() == FieldRole::Key {
                    Some(field)
                } else {
                    None
                }
            })
    }

    fn emit_key_enum_variant(&self) -> Result<TokenStream, Error> {
        let name = &self.name;
        let fields = match &self.fields {
            syn::Fields::Named(named) => syn::Fields::Named(syn::FieldsNamed {
                brace_token: named.brace_token,
                named: self.iter_key_fields().cloned().collect(),
            }),
            syn::Fields::Unnamed(unnamed) => syn::Fields::Unnamed(syn::FieldsUnnamed {
                paren_token: unnamed.paren_token,
                unnamed: self.iter_key_fields().cloned().collect(),
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
