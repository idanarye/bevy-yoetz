//! Yoetz - A Rule Based AI Plugin for the Bevy Game Engine
//!
//! Yoetz ("advisor" in Hebrew) is a rule based AI plugin for Bevy, structured around the following
//! tenants:
//!
//! 1. There is no need to build special data structures for calculating the transitions and the
//!    scores when representing these mechanisms as code inside user systems is both more flexible
//!    and simpler to use.
//! 2. The systems that check the rules need to be able to pass data to the systems act upon the
//!    decisions.
//! 3. Enacting the decision should be done with the ECS. If the action that the rules mechanism
//!    decided to do is reflected by components, it becomes easy to write different systems that
//!    perform the various possible actions.
//!
//! # Quick Start
//!
//! Define the various actions the AI can do with an enum that derives
//! [`YoetzAdvisor`](crate::advisor::YoetzAdvisor):
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoetz::prelude::*;
//! #[derive(YoetzSuggestion)]
//! enum AiBehavior {
//!     DoNothing,
//!     Attack {
//!         #[yoetz(key)]
//!         target_to_attack: Entity,
//!     },
//! }
//! ```
mod advisor;

use std::marker::PhantomData;

use bevy::prelude::*;

use self::advisor::update_advisor;
use self::prelude::YoetzSuggestion;

pub use bevy;

pub mod prelude {
    pub use crate::advisor::{YoetzAdvisor, YoetzSuggestion};
    pub use crate::{YoetzPlugin, YoetzSystemSet};
    pub use bevy_yoetz_macros::YoetzSuggestion;
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
