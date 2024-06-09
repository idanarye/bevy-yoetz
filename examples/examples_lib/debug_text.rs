use bevy::prelude::*;
use bevy::sprite::Anchor;

pub struct ExampleDebugTextPlugin;

impl Plugin for ExampleDebugTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (add_missing, update_text));
    }
}

#[derive(Component)]
pub struct ExampleDebugText {
    pub text: String,
    color: Color,
}

impl ExampleDebugText {
    pub fn new(color: Color) -> Self {
        Self {
            text: String::default(),
            color,
        }
    }
}

fn add_missing(
    query: Query<(Entity, &ExampleDebugText), Added<ExampleDebugText>>,
    mut commands: Commands,
) {
    for (entity, debug_text) in query.iter() {
        commands.entity(entity).with_children(|commands| {
            commands.spawn(Text2dBundle {
                text: Text::from_section(
                    String::default(),
                    TextStyle {
                        font: Default::default(),
                        font_size: 72.0,
                        color: debug_text.color,
                    },
                ),
                text_anchor: Anchor::BottomCenter,
                transform: Transform::from_xyz(0.0, 1.0, 1.0).with_scale(0.015 * Vec3::ONE),
                ..Default::default()
            });
        });
    }
}

fn update_text(
    mut text_query: Query<(&mut Text, &Parent)>,
    parent_query: Query<&ExampleDebugText>,
) {
    for (mut target, parent) in text_query.iter_mut() {
        let Ok(source) = parent_query.get(parent.get()) else {
            continue;
        };
        let section = &mut target.sections[0];
        section.value.clone_from(&source.text);
        section.style.color = source.color;
    }
}
