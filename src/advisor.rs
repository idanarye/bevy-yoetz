use bevy::ecs::query::{QueryData, WorldQuery};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

pub trait YoetzSuggestion: 'static + Sized + Send + Sync {
    type Key: 'static + Send + Sync + Clone + PartialEq;
    type OmniQuery: QueryData;

    fn key(&self) -> Self::Key;

    fn remove_components(key: &Self::Key, cmd: &mut EntityCommands);
    fn add_components(&self, cmd: &mut EntityCommands);
    fn update_into_components(
        self,
        components: &mut <Self::OmniQuery as WorldQuery>::Item<'_>,
    ) -> Result<(), Self>;
}

#[derive(Component)]
pub struct YoetzAdvisor<S: YoetzSuggestion> {
    consistency_bonus: f32,
    pub active_key: Option<S::Key>,
    top_suggestion: Option<(f32, S)>,
}

impl<S: YoetzSuggestion> YoetzAdvisor<S> {
    pub fn new(consistency_bonus: f32) -> Self {
        Self {
            consistency_bonus,
            active_key: None,
            top_suggestion: None,
        }
    }

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
