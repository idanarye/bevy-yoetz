mod suggestion;
mod util;

use syn::parse_macro_input;

#[proc_macro_derive(YoetzSuggestion, attributes(yoetz))]
pub fn derive_suggestion(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match suggestion::impl_suggestion(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
