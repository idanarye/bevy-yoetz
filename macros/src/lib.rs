mod suggestion;
mod util;

use syn::parse_macro_input;

/// Generate implementation `YoetzSuggestion` together with the companion types required for it.
///
/// The generated companion types are:
///
/// * The key type - with its name being the suggestion type's name concatenated with the "Key"
///   suffix. An `enum` containing each variant of the suggestion enum, but with only the fields
///   marked as `#[yoetz(key)]` included.
/// * A behavior struct for each variant - with their names being the suggestion type's name
///   concatenated with the variant's name. These structs act as Bevy `Component`s which will be
///   added to the entity when the suggested variant is chosen.
/// * For internal usage only - an omni-query struct.
///
/// This macro must decorate an `enum`, and each variant of the `enum` must be either a unit
/// variant or a struct variant (tuple variants are not allowed). Each field of a struct variant
/// must be annotated with a `#[yoetz(...)]` attribute that specifies its role:
///
/// * Key fields (annotated with `#[yoetz(key)]`) can discern between different suggestions. If the
///   same variant is suggested but with a difference in the key fields, it will be considered as a
///   different suggestion, which means the `consistency_bonus` will not be applied and that
///   components will be re-created.
/// * Input fields (annotated with `#[yoetz(input)]`) always get updated from the suggestion, even
///   if the suggestion itself (and therefore the components) do not change.
/// * State fields (annotated with `#[yoetz(state)]`) only get initialized from the suggestion when
///   the suggestion itself changes. When it doesn't (the variant and the key fields remain the
///   same) the state fields from the suggestion are discarded, which means that the action systems
///   can use them to maintain their own state.
#[proc_macro_derive(YoetzSuggestion, attributes(yoetz))]
pub fn derive_suggestion(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match suggestion::impl_suggestion(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
