use syn::Error;

use crate::util::{ApplyMeta, AttrArg};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum FieldRole {
    Key,
    Input,
    State,
}

#[derive(Default)]
pub struct FieldConfig {
    pub role: Option<FieldRole>,
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
    pub fn new_for(field: &mut syn::Field) -> Result<Self, Error> {
        let mut result = Self::default();
        for attr in field.attrs.drain(..) {
            if attr.path().is_ident("yoetz") {
                result.apply_attr(&attr)?;
            }
        }

        if result.role.is_none() {
            return Err(Error::new_spanned(&field, "YoetzSuggestion variant fields must be `#[yoets(<role>)]`, where <role> is key, input or state"));
        }

        Ok(result)
    }
}
