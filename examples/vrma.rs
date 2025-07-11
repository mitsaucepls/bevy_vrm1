//! This example shows how to animate a VRM model using VRMA.

use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
use bevy_vrm1::prelude::*;
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, VrmPlugin, VrmaPlugin))
        .add_systems(Startup, (spawn_directional_light, spawn_camera, spawn_vrm))
        .run();
}

fn spawn_directional_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(3.0, 3.0, 0.3).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(0., 0.8, 2.5)));
}

fn spawn_vrm(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn(VrmHandle(asset_server.load("vrm/AliciaSolid.vrm")))
        .with_children(|cmd| {
            // You need to spawn VRMA as a child of the VRM you want to retarget.
            cmd.spawn(VrmaHandle(asset_server.load("vrma/VRMA_01.vrma")))
                .observe(apply_play_vrma);
        });
}

/// When the VRMA animation player is set up, a [`LoadedVrma`] trigger is fired.
/// You cannot play animations until this load is complete.
fn apply_play_vrma(
    trigger: Trigger<LoadedVrma>,
    mut commands: Commands,
) {
    let vrma_entity = trigger.target();
    commands.entity(vrma_entity).trigger(PlayVrma {
        repeat: RepeatAnimation::Forever,
        transition_duration: Duration::ZERO,
    });
}
