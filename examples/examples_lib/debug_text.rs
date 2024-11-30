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
        commands.entity(entity).with_child((
            Text2d::default(),
            TextFont {
                font_size: 72.0,
                ..Default::default()
            },
            TextColor(debug_text.color),
            Anchor::BottomCenter,
            Transform::from_xyz(0.0, 1.0, 1.0).with_scale(0.015 * Vec3::ONE),
        ));
    }
}

fn update_text(
    mut text_query: Query<(&mut Text2d, &mut TextColor, &Parent)>,
    parent_query: Query<&ExampleDebugText>,
) {
    for (mut text, mut text_color, parent) in text_query.iter_mut() {
        let Ok(source) = parent_query.get(parent.get()) else {
            continue;
        };
        text.0.clone_from(&source.text);
        text_color.0 = source.color;
    }
}
