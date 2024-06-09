//! Yoetz - A Rule Based AI Plugin for the Bevy Game Engine
//!
//! Yoetz ("advisor" in Hebrew) is a rule based AI plugin for Bevy, structured around the following
//! tenets:
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
//! Define the various actions the AI can do with an enum that derives [`YoetzSuggestion`], and add
//! a [`YoetzPlugin`] for it:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoetz::prelude::*;
//! # let mut app = App::new();
//! app.add_plugins(YoetzPlugin::<AiBehavior>::new(FixedUpdate));
//!
//! #[derive(YoetzSuggestion)]
//! enum AiBehavior {
//!     DoNothing,
//!     Attack {
//!         #[yoetz(key)]
//!         target_to_attack: Entity,
//!     },
//! }
//! ```
//!
//! Give [`YoetzAdvisor`](crate::advisor::YoetzAdvisor) to the AI controlled entities:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoetz::prelude::*;
//! # let mut commands: Commands = panic!();
//! # #[derive(YoetzSuggestion)] enum AiBehavior { VariantSoThatItWontBeEmpty }
//! # #[derive(Component)] struct OtherComponentsForThisEntity;
//! commands.spawn((
//!     // The argument to `new` is a bonus for maintaining the current action.
//!     YoetzAdvisor::<AiBehavior>::new(2.0),
//!     OtherComponentsForThisEntity,
//! ));
//! ```
//!
//! Add under [`YoetzSystemSet::Suggest`] systems that check for the various rules and generate
//! suggestions with scores:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoetz::prelude::*;
//! # #[derive(YoetzSuggestion)]
//! # enum AiBehavior {
//! #     DoNothing,
//! #     Attack {
//! #         #[yoetz(key)]
//! #         target_to_attack: Entity,
//! #     },
//! # }
//! # let mut app = App::new();
//! app.add_systems(
//!     FixedUpdate,
//!     (
//!         make_ai_entities_do_nothing,
//!         give_targets_to_ai_entities,
//!     )
//!         .in_set(YoetzSystemSet::Suggest),
//! );
//!
//! fn make_ai_entities_do_nothing(mut query: Query<&mut YoetzAdvisor<AiBehavior>>) {
//!     for mut advisor in query.iter_mut() {
//!         // A constant suggestion, so that if nothing else beats this score the entity will
//!         // still have a behavior to execute.
//!         advisor.suggest(0.0, AiBehavior::DoNothing);
//!     }
//! }
//!
//! # #[derive(Component)] struct Attackable;
//! fn give_targets_to_ai_entities(
//!     mut query: Query<(&mut YoetzAdvisor<AiBehavior>, &GlobalTransform)>,
//!     targets_query: Query<(Entity, &GlobalTransform), With<Attackable>>,
//! ) {
//!     for (mut advisor, ai_transform) in query.iter_mut() {
//!         for (target_entity, target_transorm) in targets_query.iter() {
//!             let distance = ai_transform.translation().distance(target_transorm.translation());
//!             advisor.suggest(
//!                 // The closer the target, the more desirable it is to attack it. If the
//!                 // distance is more than 10, the score will get below 0 and the DoNothing
//!                 // suggestion will be used instead.
//!                 10.0 - distance,
//!                 AiBehavior::Attack {
//!                     target_to_attack: target_entity,
//!                 },
//!             );
//!         }
//!     }
//! }
//! ```
//!
//! Add under [`YoetzSystemSet::Act`] systems that performs these actions. These systems use
//! components that are generated by the [`YoetzSuggestion`](bevy_yoetz_macros::YoetzSuggestion)
//! macro and are added and removed automatically by [`YoetzPlugin`]:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoetz::prelude::*;
//! # #[derive(YoetzSuggestion)]
//! # enum AiBehavior {
//! #     DoNothing,
//! #     Attack {
//! #         #[yoetz(key)]
//! #         target_to_attack: Entity,
//! #     },
//! # }
//! # let mut app = App::new();
//! app.add_systems(
//!     FixedUpdate,
//!     (
//!         perform_do_nothing,
//!         perform_attack,
//!     )
//!         .in_set(YoetzSystemSet::Act),
//! );
//!
//! fn perform_do_nothing(query: Query<&AiBehaviorDoNothing>) {
//!     for _do_nothing in query.iter() {
//!         // Do... nothing. This whole function is kind of pointless.
//!     }
//! }
//!
//! # #[derive(Component)] struct Attacker;
//! # impl Attacker { fn attack(&mut self, _target: Entity) {} }
//! fn perform_attack(mut query: Query<(&mut Attacker, &AiBehaviorAttack)>) {
//!     for (mut attacker, attack_behavior) in query.iter_mut() {
//!         attacker.attack(attack_behavior.target_to_attack);
//!     }
//! }
mod advisor;

use std::marker::PhantomData;

use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

use self::advisor::update_advisor;
use self::prelude::YoetzSuggestion;

pub use bevy;

pub mod prelude {
    #[doc(inline)]
    pub use crate::advisor::{YoetzAdvisor, YoetzSuggestion};
    #[doc(inline)]
    pub use crate::{YoetzPlugin, YoetzSystemSet};
}

/// Add systems for processing a [`YoetzSuggestion`].
pub struct YoetzPlugin<S: YoetzSuggestion> {
    schedule: InternedScheduleLabel,
    _phantom: PhantomData<fn(S)>,
}

impl<S: YoetzSuggestion> YoetzPlugin<S> {
    /// Create a `YoetzPlugin` that cranks the [`YoetzAdvisor`](crate::advisor::YoetzAdvisor) in
    /// the given schedule.
    ///
    /// The update will be done between [`YoetzSystemSet::Suggest`] and [`YoetzSystemSet::Act`] in
    /// that schedule.
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
            _phantom: PhantomData,
        }
    }
}

impl<S: 'static + YoetzSuggestion> Plugin for YoetzPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            self.schedule,
            (
                YoetzSystemSet::Suggest,
                YoetzInternalSystemSet::Think,
                YoetzSystemSet::Act,
            )
                .chain(),
        );
        app.add_systems(
            self.schedule,
            update_advisor::<S>.in_set(YoetzInternalSystemSet::Think),
        );
    }
}

/// System sets to put suggestion systems and action systems in.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub enum YoetzSystemSet {
    /// Systems that suggest behaviors (by calling
    /// [`YoetzAdvisor::suggest`](advisor::YoetzAdvisor::suggest)) should go in this set.
    Suggest,
    /// Systems that enact behaviors (by querying for the behavior structs generated by the
    /// [`YoetzSuggestion`](bevy_yoetz_macros::YoetzSuggestion) macro) should go in this set.
    Act,
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub enum YoetzInternalSystemSet {
    Think,
}
