use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Error;

use crate::util::{ApplyMeta, AttrArg};

use super::field::FieldRole;
use super::generated_type::GeneratedTypeConfig;
use super::variant::SuggestionVariantData;

pub struct SuggestionEnumData {
    pub visibility: syn::Visibility,
    pub name: syn::Ident,
    pub key_enum_name: syn::Ident,
    pub omni_query_name: syn::Ident,
    pub key_enum_config: GeneratedTypeConfig,
    pub strategy_structs_config: GeneratedTypeConfig,
}

impl TryFrom<&syn::DeriveInput> for SuggestionEnumData {
    type Error = Error;

    fn try_from(ast: &syn::DeriveInput) -> Result<Self, Self::Error> {
        let mut result = Self {
            visibility: ast.vis.clone(),
            name: ast.ident.clone(),
            key_enum_name: syn::Ident::new(&format!("{}Key", ast.ident), ast.ident.span()),
            omni_query_name: syn::Ident::new(&format!("{}OmniQuery", ast.ident), ast.ident.span()),
            key_enum_config: GeneratedTypeConfig::default(),
            strategy_structs_config: GeneratedTypeConfig::default(),
        };
        for attr in ast.attrs.iter() {
            if attr.path().is_ident("yoetz") {
                result.apply_attr(attr)?;
            }
        }
        Ok(result)
    }
}

impl ApplyMeta for SuggestionEnumData {
    fn apply_meta(&mut self, expr: AttrArg) -> Result<(), Error> {
        match expr.name().to_string().as_str() {
            "key_enum" => self.key_enum_config.apply_sub_attr(expr.sub_attr()?),
            "strategy_structs" => self
                .strategy_structs_config
                .apply_sub_attr(expr.sub_attr()?),
            _ => Err(expr.unknown_name()),
        }
    }
}

impl SuggestionEnumData {
    pub fn emit_key_enum_code(
        &self,
        variants: &[SuggestionVariantData],
    ) -> Result<TokenStream, Error> {
        let visibility = &self.visibility;
        let key_enum_name = &self.key_enum_name;
        let variant_options = variants
            .iter()
            .map(|variant| variant.emit_key_enum_variant())
            .collect::<Result<Vec<_>, _>>()?;
        let extra_derives = &self.key_enum_config.derive;
        Ok(quote! {
            #[derive(Clone, PartialEq, #(#extra_derives),*)]
            #visibility enum #key_enum_name {
                #(#variant_options,)*
            }
        })
    }

    pub fn emit_omni_query_code(
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

    pub fn emit_trait_impl(
        &self,
        variants: &[SuggestionVariantData],
    ) -> Result<TokenStream, Error> {
        let Self {
            visibility: _,
            name: suggestion_enum_name,
            key_enum_name,
            omni_query_name,
            key_enum_config: _,
            strategy_structs_config: _,
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

    pub fn emit_key_method(
        &self,
        variants: &[SuggestionVariantData],
    ) -> Result<TokenStream, Error> {
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
                syn::Fields::Unnamed(_) => panic!("currently unsupported"),
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
                            #(#fields),*
                        });
                    }
                },
                syn::Fields::Unnamed(_) => panic!("currently unsupported"),
                syn::Fields::Unit => quote! {
                    #suggestion_enum_name::#variant_name => {
                        cmd.insert(#strategy_name);
                    }
                },
            });
        }

        Ok(quote! {
            fn add_components(self, cmd: &mut bevy::ecs::system::EntityCommands) {
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
                syn::Fields::Unnamed(_) => panic!("currently unsupported"),
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
