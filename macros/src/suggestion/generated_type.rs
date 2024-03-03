use syn::Error;

use crate::util::{ApplyMeta, AttrArg};

#[derive(Default)]
pub struct GeneratedTypeConfig {
    pub derive: Vec<syn::Path>,
}

impl ApplyMeta for GeneratedTypeConfig {
    fn apply_meta(&mut self, expr: AttrArg) -> Result<(), Error> {
        match expr.name().to_string().as_str() {
            "derive" => {
                self.derive.extend(expr.sub_attr()?.args()?);
                Ok(())
            }
            _ => Err(expr.unknown_name()),
        }
    }
}
