use bevy::{color::palettes::css, prelude::*};
use bevy_yoetz::prelude::*;
use turborand::rng::Rng;
use turborand::TurboRand;

use self::examples_lib::debug_text::{ExampleDebugText, ExampleDebugTextPlugin};

mod examples_lib;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // We need to add this YoetzPlugin for each YoetzSuggestion enum we'll use.
        .add_plugins(YoetzPlugin::<EnemyBehavior>::new(FixedUpdate))
        .add_plugins(ExampleDebugTextPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            FixedUpdate,
            (
                // These systems look at the state of the game and create scored suggestions
                // for the AI to consider.
                enemies_idle,
                enemies_detect_player,
                enemies_in_distance_for_circling,
            )
                .in_set(YoetzSystemSet::Suggest),
        )
        .add_systems(
            Update,
            (
                // These systems match with the strategies the AI decided on, and enact them.
                control_player,
                enemies_do_nothing,
                enemies_follow_player,
                enemies_circle_player,
            )
                .in_set(YoetzSystemSet::Act),
        )
        .add_systems(
            Update,
            update_enemies_debug_text.in_set(YoetzSystemSet::Act),
        )
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_scale(0.05 * Vec3::ONE),
        ..Default::default()
    });

    commands.spawn((
        Player,
        SpriteBundle {
            sprite: Sprite {
                color: css::YELLOW.into(),
                custom_size: Some(Vec2::new(1.0, 1.0)),
                ..Default::default()
            },
            ..Default::default()
        },
    ));

    commands.spawn((
        Enemy,
        // This means that this entity will have a consistency bonus of 2.0 - so a new suggestion's
        // score needs to be at least 2.0 better than the currently active one in order to replace
        // it.
        YoetzAdvisor::<EnemyBehavior>::new(2.0),
        SpriteBundle {
            transform: Transform::from_xyz(-5.0, 5.0, 0.0),
            sprite: Sprite {
                color: css::RED.into(),
                custom_size: Some(Vec2::new(1.0, 1.0)),
                ..Default::default()
            },
            ..Default::default()
        },
        ExampleDebugText::new(css::WHITE.into()),
    ));
}

fn control_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::ArrowUp) {
        direction += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        direction -= Vec3::Y;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        direction -= Vec3::X;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        direction += Vec3::X;
    }

    for mut player_transform in query.iter_mut() {
        player_transform.translation += 10.0 * time.delta_seconds() * direction;
    }
}

#[derive(YoetzSuggestion)]
#[yoetz(key_enum(derive(Debug)), strategy_structs(derive(Debug)))]
enum EnemyBehavior {
    Idle,
    Chase {
        // Having the target entity as a key field means that if there were multiple targets then
        // the suggestion to Chase each of them would be considered a different suggestion. This is
        // mainly important here because of the consistency bonus - only the suggestion to keep
        // chasing the current target gets the bonus, so the AI won't frantically switch targets.
        #[yoetz(key)]
        target_entity: Entity,
        // Technically the enemies_follow_player system could have read this value from the
        // GlobalTransform of the target (since it has the Entity), but making this an input field
        // exempts that system from having to add a second query.
        #[yoetz(input)]
        vec_to_target: Vec2,
    },
    Circle {
        #[yoetz(key)]
        target_entity: Entity,
        #[yoetz(input)]
        vec_to_target: Vec2,
        // State fields can also be used like this - the field's value is randomized each frame,
        // but only in the frame where the suggestion gets chosen it becomes a "state" and gets
        // frozen (until a different suggestion replaces the entire strategy component). This
        // ensures that each time an enemy starts to circle a target, it'll choose a random
        // direction to circle - and stick with it.
        #[yoetz(state)]
        go_counter_clockwise: bool,
    },
}

fn enemies_idle(mut query: Query<&mut YoetzAdvisor<EnemyBehavior>, With<Enemy>>) {
    for mut advisor in query.iter_mut() {
        advisor.suggest(5.0, EnemyBehavior::Idle);
    }
}

fn enemies_detect_player(
    mut enemies_query: Query<(&mut YoetzAdvisor<EnemyBehavior>, &GlobalTransform), With<Enemy>>,
    player_query: Query<(Entity, &GlobalTransform), With<Player>>,
) {
    for (mut advisor, enemy_transform) in enemies_query.iter_mut() {
        let enemy_position = enemy_transform.translation();
        for (player_entity, player_transform) in player_query.iter() {
            let player_position = player_transform.translation();
            let vec_to_player = (player_position - enemy_position).truncate();
            advisor.suggest(
                // The score will go lower the farther the enemy is from the player. If it gets too
                // far, the score will go below 5.0 - and the Idle strategy will kick in. Of
                // course, the consistency bonus also applies.
                12.0 - vec_to_player.length(),
                EnemyBehavior::Chase {
                    target_entity: player_entity,
                    vec_to_target: vec_to_player,
                },
            );
        }
    }
}

fn enemies_in_distance_for_circling(
    mut enemies_query: Query<(&mut YoetzAdvisor<EnemyBehavior>, &GlobalTransform), With<Enemy>>,
    player_query: Query<(Entity, &GlobalTransform), With<Player>>,
    rand: Local<Rng>,
) {
    for (mut advisor, enemy_transform) in enemies_query.iter_mut() {
        let enemy_position = enemy_transform.translation();
        for (player_entity, player_transform) in player_query.iter() {
            let player_position = player_transform.translation();
            let vec_to_player = (player_position - enemy_position).truncate();
            if vec_to_player.length() < 4.0 {
                advisor.suggest(
                    // Give it a high score because we already filter for it. If we are within
                    // range, no other suggestion should beat it.
                    100.0,
                    EnemyBehavior::Circle {
                        target_entity: player_entity,
                        vec_to_target: vec_to_player,
                        go_counter_clockwise: rand.bool(),
                    },
                );
            }
        }
    }
}

fn enemies_do_nothing(query: Query<&EnemyBehaviorIdle>) {
    for _ in query.iter() {
        // Nothing really to do here. I didn't actually have to add this system...
    }
}

fn enemies_follow_player(mut query: Query<(&EnemyBehaviorChase, &mut Transform)>, time: Res<Time>) {
    for (chase, mut transform) in query.iter_mut() {
        let Some(direction) = chase.vec_to_target.try_normalize() else {
            continue;
        };
        transform.translation += 5.0 * time.delta_seconds() * direction.extend(0.0);
    }
}

fn enemies_circle_player(
    mut query: Query<(&EnemyBehaviorCircle, &mut Transform)>,
    time: Res<Time>,
) {
    for (circle, mut transform) in query.iter_mut() {
        let Some(direction) = circle.vec_to_target.try_normalize() else {
            continue;
        };
        let direction = if circle.go_counter_clockwise {
            direction.perp()
        } else {
            -direction.perp()
        };
        transform.translation -= 5.0 * time.delta_seconds() * direction.extend(0.0);
    }
}

#[allow(clippy::type_complexity)]
fn update_enemies_debug_text(
    mut query: Query<(
        &mut ExampleDebugText,
        &YoetzAdvisor<EnemyBehavior>,
        Option<&EnemyBehaviorIdle>,
        Option<&EnemyBehaviorChase>,
        Option<&EnemyBehaviorCircle>,
    )>,
) {
    for (mut debug_text, advisor, idle, chase, circle) in query.iter_mut() {
        use std::fmt::Write;
        let mut text = String::default();

        fn write_if_some(sink: &mut String, value: Option<impl std::fmt::Debug>) {
            if let Some(value) = value {
                writeln!(sink, "{value:#?}").unwrap();
            }
        }

        // We can tell the currently active suggestion using the active_key, and because we made
        // the key enum `derive(Debug)` we can write it to the debug text.
        write_if_some(&mut text, advisor.active_key().as_ref());
        write_if_some(&mut text, idle);
        write_if_some(&mut text, chase);
        write_if_some(&mut text, circle);

        debug_text.text = text;
    }
}
