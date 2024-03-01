mod advisor;

use std::marker::PhantomData;

use bevy::prelude::*;

use self::advisor::update_advisor;
use self::prelude::YoetzSuggestion;

pub mod prelude {
    pub use crate::advisor::{YoetzAdvisor, YoetzSuggestion};
    pub use crate::{YoetzPlugin, YoetzSystemSet};
}

pub struct YoetzPlugin<S: YoetzSuggestion> {
    _phantom: PhantomData<fn(S)>,
}

impl<S: YoetzSuggestion> Default for YoetzPlugin<S> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<S: 'static + YoetzSuggestion> Plugin for YoetzPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                YoetzSystemSet::Suggest,
                YoetzInternalSystemSet::Think,
                YoetzSystemSet::Act,
            )
                .chain(),
        );
        app.add_systems(
            Update,
            update_advisor::<S>.in_set(YoetzInternalSystemSet::Think),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub enum YoetzSystemSet {
    Suggest,
    Act,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub enum YoetzInternalSystemSet {
    Think,
}
