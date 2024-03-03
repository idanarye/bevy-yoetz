use proc_macro2::{Span, TokenStream};
use syn::Error;

use self::suggestion_enum::SuggestionEnumData;
use self::variant::SuggestionVariantData;

mod field;
mod generated_type;
mod suggestion_enum;
mod variant;

pub fn impl_suggestion(ast: &syn::DeriveInput) -> Result<TokenStream, Error> {
    let syn::Data::Enum(ast_enum) = &ast.data else {
        return Err(Error::new(
            Span::call_site(),
            "YoetzSuggestion can only be derived from an enum",
        ));
    };
    let enum_data = SuggestionEnumData::try_from(ast)?;
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
