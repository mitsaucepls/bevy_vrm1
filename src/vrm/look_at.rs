//! - [`look at specification(en)`](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/lookAt.md)
//! - [`look at specification(ja)`](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/lookAt.ja.md)

use crate::prelude::*;
use crate::system_set::VrmSystemSets;
use bevy::app::{Animation, App, Plugin};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::TransformSystem::TransformPropagate;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::window::{PrimaryWindow, WindowRef};

/// Holds the entity of looking the target entity.
/// This component should be inserted into the root entity of the VRM.
///
/// [`LookAt::Cursor`] is used to look at the mouse cursor in the window.
/// [`LookAt::Target`] is used to look at the specified entity.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_vrm1::prelude::*;
///
/// fn spawn_camera_and_vrm(
///     mut commands: Commands,
///     asset_server: Res<AssetServer>,
/// ) {
///     let camera = commands.spawn(Camera3d::default()).id();
///     commands.spawn((
///         VrmHandle(asset_server.load("model.vrm")),
///         LookAt::Cursor {
///             camera: Some(camera),
///         },
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub enum LookAt {
    /// Look at the window cursor.
    /// The camera entity that is specified as the render target of the window must be passed.
    /// If `None`, it searches all cameras and uses the cursor position of the first one found.
    Cursor { camera: Option<Entity> },

    /// Specify the entity of the target.
    Target(Entity),
}

pub(super) struct LookAtPlugin;

impl Plugin for LookAtPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<LookAt>()
            .register_type::<LookAtProperties>()
            .register_type::<LookAtType>()
            .add_systems(
                PostUpdate,
                track_looking_target
                    .run_if(on_event::<MouseMotion>)
                    .in_set(VrmSystemSets::LookAt)
                    .after(Animation)
                    .after(TransformPropagate),
            );
    }
}

fn track_looking_target(
    mut commands: Commands,
    vrms: Query<(
        &LookAt,
        &LookAtProperties,
        &HeadBoneEntity,
        &LeftEyeBoneEntity,
        &RightEyeBoneEntity,
    )>,
    cameras: Query<(Entity, &Camera)>,
    transforms: Query<&Transform>,
    global_transforms: Query<&GlobalTransform>,
    windows: Query<(&Window, Has<PrimaryWindow>)>,
) {
    vrms.iter()
        .for_each(|(look_at, properties, head, left_eye, right_eye)| {
            let Ok(head_gtf) = global_transforms.get(head.0) else {
                return;
            };
            let Ok(head_tf) = transforms.get(head.0) else {
                return;
            };
            let look_at_space = GlobalTransform::default();
            let mut look_at_space_tf = look_at_space.reparented_to(head_gtf);
            look_at_space_tf.translation = Vec3::from(properties.offset_from_head_bone);
            look_at_space_tf.rotation = head_tf.rotation.inverse();
            let look_at_space = head_gtf.mul_transform(look_at_space_tf);
            let Some(target) = calc_target_position(
                look_at,
                head.0,
                &transforms,
                &global_transforms,
                &cameras,
                &windows,
            ) else {
                return;
            };
            let (yaw, pitch) = calc_yaw_pitch(&look_at_space, target);
            match properties.r#type {
                LookAtType::Bone => {
                    apply_bone(
                        &mut commands,
                        &transforms,
                        left_eye,
                        right_eye,
                        properties,
                        yaw,
                        pitch,
                    );
                }
                LookAtType::Expression => {
                    todo!("Expression look at is not supported yet");
                }
            }
        });
}

fn calc_target_position(
    look_at: &LookAt,
    vrm_entity: Entity,
    transforms: &Query<&Transform>,
    global_transforms: &Query<&GlobalTransform>,
    cameras: &Query<(Entity, &Camera)>,
    windows: &Query<(&Window, Has<PrimaryWindow>)>,
) -> Option<Vec3> {
    match look_at {
        LookAt::Cursor { camera } => match camera {
            Some(camera_entity) => calc_look_at_cursor_position(
                *camera_entity,
                vrm_entity,
                global_transforms,
                cameras,
                windows,
            ),
            None => cameras.iter().find_map(|(camera_entity, _)| {
                calc_look_at_cursor_position(
                    camera_entity,
                    vrm_entity,
                    global_transforms,
                    cameras,
                    windows,
                )
            }),
        },
        LookAt::Target(target_entity) => transforms.get(*target_entity).map(|t| t.translation).ok(),
    }
}

fn apply_bone(
    commands: &mut Commands,
    transforms: &Query<&Transform>,
    left_eye: &LeftEyeBoneEntity,
    right_eye: &RightEyeBoneEntity,
    properties: &LookAtProperties,
    yaw: f32,
    pitch: f32,
) {
    let Ok(left_eye_tf) = transforms.get(left_eye.0) else {
        return;
    };
    let Ok(right_eye_tf) = transforms.get(right_eye.0) else {
        return;
    };
    let applied_left_eye_tf = apply_left_eye_bone(left_eye_tf, properties, yaw, pitch);
    let applied_right_eye_tf = apply_right_eye_bone(right_eye_tf, properties, yaw, pitch);
    commands.entity(left_eye.0).insert(applied_left_eye_tf);
    commands.entity(right_eye.0).insert(applied_right_eye_tf);
}

fn calc_look_at_cursor_position(
    camera_entity: Entity,
    vrm_entity: Entity,
    global_transforms: &Query<&GlobalTransform>,
    cameras: &Query<(Entity, &Camera)>,
    windows: &Query<(&Window, Has<PrimaryWindow>)>,
) -> Option<Vec3> {
    let (_, camera) = cameras.get(camera_entity).ok()?;
    let camera_gtf = global_transforms.get(camera_entity).ok()?;
    let head_gtf = global_transforms.get(vrm_entity).ok()?;
    let RenderTarget::Window(window_ref) = camera.target else {
        return None;
    };
    let window = match window_ref {
        WindowRef::Primary => windows
            .iter()
            .find_map(|(w, primary)| primary.then_some(w))?,
        WindowRef::Entity(window_entity) => windows.get(window_entity).map(|(w, _)| w).ok()?,
    };

    let cursor = window.cursor_position()?;

    let ray = camera.viewport_to_world(camera_gtf, cursor).ok()?;
    let delta = camera_gtf.translation() - head_gtf.translation();
    let plane_origin = head_gtf.translation() + delta * 0.5;
    let plane_up = InfinitePlane3d::new(camera_gtf.back());
    let distance = ray.intersect_plane(plane_origin, plane_up)?;

    Some(ray.get_point(distance))
}

fn calc_yaw_pitch(
    look_at_space: &GlobalTransform,
    target: Vec3,
) -> (f32, f32) {
    let local_target = look_at_space
        .compute_matrix()
        .inverse()
        .transform_point3(target);

    let z = local_target.dot(Vec3::Z);
    let x = local_target.dot(Vec3::X);
    let yaw = (x.atan2(z)).to_degrees();

    let xz = (x * x + z * z).sqrt();
    let y = local_target.dot(Vec3::Y);
    let pitch = (-y.atan2(xz)).to_degrees();

    (yaw, pitch)
}

fn apply_left_eye_bone(
    left_eye: &Transform,
    properties: &LookAtProperties,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> Transform {
    let range_map_horizontal_outer = properties.range_map_horizontal_outer;
    let range_map_horizontal_inner = properties.range_map_horizontal_inner;
    let range_map_vertical_down = properties.range_map_vertical_down;
    let range_map_vertical_up = properties.range_map_vertical_up;
    let yaw = if yaw_degrees > 0.0 {
        yaw_degrees.min(range_map_horizontal_outer.input_max_value)
            / range_map_horizontal_outer.input_max_value
            * range_map_horizontal_outer.output_scale
    } else {
        -(yaw_degrees
            .abs()
            .min(range_map_horizontal_inner.input_max_value)
            / range_map_horizontal_inner.input_max_value
            * range_map_horizontal_inner.output_scale)
    };

    let pitch = if pitch_degrees > 0.0 {
        pitch_degrees.min(range_map_vertical_down.input_max_value)
            / range_map_vertical_down.input_max_value
            * range_map_vertical_down.output_scale
    } else {
        -(pitch_degrees
            .abs()
            .min(range_map_vertical_up.input_max_value)
            / range_map_vertical_up.input_max_value
            * range_map_vertical_up.output_scale)
    };
    left_eye.with_rotation(Quat::from_euler(
        EulerRot::YXZ,
        yaw.to_radians(),
        pitch.to_radians(),
        0.0,
    ))
}

fn apply_right_eye_bone(
    right_eye: &Transform,
    properties: &LookAtProperties,
    yaw_degress: f32,
    pitch_degress: f32,
) -> Transform {
    let range_map_horizontal_outer = properties.range_map_horizontal_outer;
    let range_map_horizontal_inner = properties.range_map_horizontal_inner;
    let range_map_vertical_down = properties.range_map_vertical_down;
    let range_map_vertical_up = properties.range_map_vertical_up;

    let yaw = if yaw_degress > 0.0 {
        yaw_degress.min(range_map_horizontal_inner.input_max_value)
            / range_map_horizontal_inner.input_max_value
            * range_map_horizontal_inner.output_scale
    } else {
        -(yaw_degress
            .abs()
            .min(range_map_horizontal_outer.input_max_value)
            / range_map_horizontal_outer.input_max_value
            * range_map_horizontal_outer.output_scale)
    };

    let pitch = if pitch_degress > 0.0 {
        pitch_degress.min(range_map_vertical_down.input_max_value)
            / range_map_vertical_down.input_max_value
            * range_map_vertical_down.output_scale
    } else {
        -(pitch_degress
            .abs()
            .min(range_map_vertical_up.input_max_value)
            / range_map_vertical_up.input_max_value
            * range_map_vertical_up.output_scale)
    };

    right_eye.with_rotation(Quat::from_euler(
        EulerRot::YXZ,
        yaw.to_radians(),
        pitch.to_radians(),
        0.0,
    ))
}
