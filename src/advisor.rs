use bevy::ecs::query::QueryData;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

#[doc(inline)]
pub use bevy_yoetz_macros::YoetzSuggestion;

/// An action suggestion for the AI agent to consider.
///
/// Avoid implementing this trait manually - prefer using the
/// [`YoetzSuggestion`](bevy_yoetz_macros::YoetzSuggestion) derive macro.
///
/// `enum`s that implement this trait are mainly used as the generic parameter for [`YoetzAdvisor`]
/// and as the data passed to it. A [`YoetzPlugin`](crate::YoetzPlugin) parametrized on them should
/// also be added to the Bevy application.
pub trait YoetzSuggestion: 'static + Sized + Send + Sync {
    /// The key identifies a suggestion even when its data changes. The
    /// [`YoetzSuggestion`](bevy_yoetz_macros::YoetzSuggestion) derive macro generates a key that
    /// is a "subset" of the `enum` - it contains all the variants, but each variant only contains
    /// the fields marked as `#[yoetz(key)]`.
    type Key: 'static + Send + Sync + Clone + PartialEq;

    /// A query that allows access to all possible behavior components.
    ///
    /// The query generated by the [`YoetzSuggestion`](bevy_yoetz_macros::YoetzSuggestion) derive
    /// macro is unsightly and there it never a reason to use it manually.
    type OmniQuery: QueryData;

    /// Generate a [`Key`](Self::Key) that identifies the suggestion.
    fn key(&self) -> Self::Key;

    /// Remove the behavior components that were created by a suggestion with the specified key.
    fn remove_components(key: &Self::Key, cmd: &mut EntityCommands);

    /// Add behavior components created from the suggestion.
    fn add_components(self, cmd: &mut EntityCommands);

    /// Update the existing behavior components from the suggestion's data.
    ///
    /// The method generated by the [`YoetzSuggestion`](bevy_yoetz_macros::YoetzSuggestion) derive
    /// macro will only update the fields marked with `#[yoetz(input)]`. Fields marked with
    /// `#[yoetz(state)]` will not be updated because the action systems are allowed to store their
    /// own state there (or just maintain the initial state from when the behavior was chosen), and
    /// fields marked with `#[yoetz(key)]` will not be updated because when they change the
    /// [`Key`](Self::Key) changes and the components themselves will be re-inserted rather than
    /// updated.
    fn update_into_components(
        self,
        components: &mut <Self::OmniQuery as QueryData>::Item<'_>,
    ) -> Result<(), Self>;
}

/// Controls an entity's AI by listening to [`YoetzSuggestion`]s and updating the entity's behavior
/// components.
#[derive(Component)]
pub struct YoetzAdvisor<S: YoetzSuggestion> {
    /// Added to score of any suggestion that matches the currently active behavior. This can be
    /// used to reduce the "flickering" when multiple suggestions are flocking around the same
    /// score.
    pub consistency_bonus: f32,
    active_key: Option<S::Key>,
    top_suggestion: Option<(f32, S)>,
}

impl<S: YoetzSuggestion> YoetzAdvisor<S> {
    /// Create a new advisor with the specified [`consistency_bonus`](Self::consistency_bonus).
    pub fn new(consistency_bonus: f32) -> Self {
        Self {
            consistency_bonus,
            active_key: None,
            top_suggestion: None,
        }
    }

    /// The [`Key`](YoetzSuggestion::Key) of the currently active behavior.
    ///
    /// This can be used to implement a state machine behavior, where the code that suggests a
    /// behavior can look at the current state.
    pub fn active_key(&self) -> &Option<S::Key> {
        &self.active_key
    }

    /// Suggest a behavior for the AI to consider.
    ///
    /// A suggestion should be sent every frame as long as it is valid - once it stops being sent
    /// it will immediately be replaced by another suggestion.
    pub fn suggest(&mut self, score: f32, suggestion: S) {
        if let Some((current_score, _)) = self.top_suggestion.as_ref() {
            let bonus = if self
                .active_key
                .as_ref()
                .map(|key| *key == suggestion.key())
                .unwrap_or(false)
            {
                self.consistency_bonus
            } else {
                0.0
            };
            if score + bonus < *current_score {
                return;
            }
        }
        self.top_suggestion = Some((score, suggestion));
    }
}

pub fn update_advisor<S: YoetzSuggestion>(
    mut query: Query<(Entity, &mut YoetzAdvisor<S>, S::OmniQuery)>,
    mut commands: Commands,
) {
    for (entity, mut advisor, mut components) in query.iter_mut() {
        let Some((_, mut suggestion)) = advisor.top_suggestion.take() else {
            continue;
        };
        let key = suggestion.key();
        let mut cmd;
        if let Some(old_key) = advisor.active_key.as_ref() {
            if *old_key == key {
                let update_result = suggestion.update_into_components(&mut components);
                if let Err(update_result) = update_result {
                    warn!(
                        "Components were wrong - will not update, add them with a command instead"
                    );
                    suggestion = update_result;
                } else {
                    continue;
                }
            }
            cmd = commands.entity(entity);
            S::remove_components(old_key, &mut cmd)
        } else {
            cmd = commands.entity(entity);
        }
        suggestion.add_components(&mut cmd);
        advisor.active_key = Some(key);
    }
}
