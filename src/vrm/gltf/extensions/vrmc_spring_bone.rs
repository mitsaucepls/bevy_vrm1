use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct VRMCSpringBone {
    /// Represents the specification version of the `VRMC_springBone` extension.
    #[serde(rename = "specVersion")]
    pub spec_version: String,

    /// [Collider]
    pub colliders: Vec<Collider>,

    /// [`ColliderGroup`]
    #[serde(rename = "colliderGroups")]
    pub collider_groups: Vec<ColliderGroup>,

    /// [Spring]
    pub springs: Vec<Spring>,
}

impl VRMCSpringBone {
    pub fn all_joints(&self) -> Vec<SpringJoint> {
        self.springs
            .iter()
            .flat_map(|spring| spring.joints.clone())
            .collect()
    }

    pub fn spring_colliders(
        &self,
        collider_group_indices: &[usize],
    ) -> Vec<Collider> {
        collider_group_indices
            .iter()
            .flat_map(|index| self.collider_groups[*index].colliders.clone())
            .flat_map(|index| self.colliders.get(index as usize).cloned())
            .collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ColliderGroup {
    /// Group name
    pub name: String,

    /// The list of colliders belonging to this group.
    /// Each value is an index of `VRMCSpringBone::colliders`.
    pub colliders: Vec<u64>,
}

/// Represents the collision detection for spring bone.
/// It consists of the target node index and the collider shape.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Collider {
    pub node: usize,
    pub shape: ColliderShape,
}

#[derive(Serialize, Deserialize)]
pub struct Spring {
    /// Spring name
    pub name: String,

    /// The list of joints that make up the springBone.
    pub joints: Vec<SpringJoint>,

    /// Each value is an index of `VRMCSpringBone::colliderGroups`.
    #[serde(rename = "colliderGroups")]
    pub collider_groups: Option<Vec<usize>>,

    pub center: Option<usize>,
}

/// The node of a single glTF with spring bone settings.
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct SpringJoint {
    pub node: usize,
    #[serde(rename = "dragForce")]
    pub drag_force: Option<f32>,
    #[serde(rename = "gravityDir")]
    pub gravity_dir: Option<[f32; 3]>,
    #[serde(rename = "gravityPower")]
    pub gravity_power: Option<f32>,
    #[serde(rename = "hitRadius")]
    pub hit_radius: Option<f32>,
    pub stiffness: Option<f32>,
}

/// The shape of the collision detection for [Collider]
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColliderShape {
    Sphere(Sphere),
    Capsule(Capsule),
}

impl Default for ColliderShape {
    fn default() -> Self {
        Self::Sphere(Sphere::default())
    }
}

impl ColliderShape {
    /// Returns the collision vector from the collider to the target position.
    pub fn apply_collision(
        &self,
        next_tail: &mut Vec3,
        collider: &GlobalTransform,
        head_global_pos: Vec3,
        joint_radius: f32,
        bone_length: f32,
    ) {
        let (scale, _, _) = collider.to_scale_rotation_translation();
        let max_collider_scale = scale.abs().max_element();
        match self {
            Self::Sphere(sphere) => {
                let translation = collider.transform_point(Vec3::from(sphere.offset));
                let r = joint_radius + sphere.radius * max_collider_scale;
                let delta = *next_tail - translation;
                let distance_squared = delta.length_squared();
                if distance_squared > 0.0 && distance_squared <= r * r {
                    let dir = delta.normalize();
                    let pos_from_collider = translation + dir * r;
                    *next_tail = head_global_pos
                        + (pos_from_collider - head_global_pos).normalize() * bone_length;
                }
            }
            Self::Capsule(_) => {
                //TODO: Not supported yet
            }
        }
    }

    #[inline]
    pub const fn radius(&self) -> f32 {
        match self {
            Self::Sphere(sphere) => sphere.radius,
            Self::Capsule(capsule) => capsule.radius,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Component, Reflect, Default)]
#[reflect(Component, Serialize, Deserialize)]
pub struct Sphere {
    /// Local coordinate of the sphere center
    pub offset: [f32; 3],
    /// Radius of the sphere
    pub radius: f32,
}

/// 楕円形の
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Component, Reflect, Default)]
#[reflect(Component, Serialize, Deserialize)]
pub struct Capsule {
    /// Local coordinate of the center of the half sphere at the start point of the capsule
    pub offset: [f32; 3],
    /// Radius of the half sphere and cylinder part of the capsule
    pub radius: f32,
    /// Local coordinate of the center of the half sphere at the end point of the capsule
    pub tail: [f32; 3],
}

#[cfg(test)]
mod tests {
    use crate::success;
    use crate::tests::TestResult;
    use crate::vrm::gltf::extensions::vrmc_spring_bone::VRMCSpringBone;

    #[test]
    fn deserialize_vrmc_spring_bone() -> TestResult {
        let _spring_bone: VRMCSpringBone =
            serde_json::from_str(include_str!("vrmc_spring_bone.json"))?;
        success!()
    }
}
