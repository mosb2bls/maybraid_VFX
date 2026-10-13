//! Far-band primitive massing for [`crate::ComponentsOnly`] buildings.
//!
//! Each [`MassingVolume`] carries its own height, primitive, and optional
//! [`MaterialRef`]. Low draws every volume; UltraLow drops accessory volumes and
//! roof pitches. Shared unit meshes must be initialized by [`MassingSilhouettePlugin`].

use std::sync::OnceLock;

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::bounding::Aabb2d;
use lod::scene::LodSceneLevel;
use lod::LodLazyPending;
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};

use crate::scene_children::scene_children;
use crate::BuildingComponents;

const MIN_EDGE: f32 = 0.25;
const DEFAULT_WALL: &str = "stucco";
const DEFAULT_ROOF: &str = "hay";

/// Primitive drawn for one authored massing volume.
#[derive(Debug, Clone, PartialEq)]
pub enum MassingKind {
	Cuboid,
	Cylinder,
	/// Axis-aligned frustum. `ridge` is the full XZ extent at the top.
	Trazaloid {
		ridge: Vec2,
	},
	/// Triangular prism. Ridge runs along X when `along_x`, otherwise along Z.
	RoofPitch {
		along_x: bool,
	},
}

/// Optional pitched cap sitting on a wall volume (not extra wall height).
#[derive(Debug, Clone, PartialEq)]
pub struct MassingRoof {
	pub rise: f32,
	pub along_x: bool,
	pub material: Option<MaterialRef>,
}

/// One far-band primitive in the host's local frame.
#[derive(Debug, Clone, PartialEq)]
pub struct MassingVolume {
	pub kind: MassingKind,
	/// Plan rectangle (`Aabb2d.y` = world \(z\)).
	pub xz: Aabb2d,
	pub y0: f32,
	pub height: f32,
	pub material: Option<MaterialRef>,
	pub roof: Option<MassingRoof>,
	/// Stairwells / colonnades — omitted at UltraLow.
	pub accessory: bool,
}

impl MassingVolume {
	pub fn cuboid(xz: Aabb2d, y0: f32, height: f32) -> Self {
		Self {
			kind: MassingKind::Cuboid,
			xz,
			y0,
			height: height.max(MIN_EDGE),
			material: None,
			roof: None,
			accessory: false,
		}
	}

	pub fn cylinder(center_xz: Vec2, radius: f32, y0: f32, height: f32) -> Self {
		let r = radius.max(MIN_EDGE);
		Self {
			kind: MassingKind::Cylinder,
			xz: Aabb2d { min: center_xz - Vec2::splat(r), max: center_xz + Vec2::splat(r) },
			y0,
			height: height.max(MIN_EDGE),
			material: None,
			roof: None,
			accessory: false,
		}
	}

	pub fn trazaloid(origin: Vec3, footprint: Vec2, ridge: Vec2, height: f32) -> Self {
		let half = footprint.max(Vec2::splat(MIN_EDGE)) * 0.5;
		Self {
			kind: MassingKind::Trazaloid { ridge: ridge.max(Vec2::splat(MIN_EDGE)) },
			xz: Aabb2d {
				min: Vec2::new(origin.x - half.x, origin.z - half.y),
				max: Vec2::new(origin.x + half.x, origin.z + half.y),
			},
			y0: origin.y,
			height: height.max(MIN_EDGE),
			material: None,
			roof: None,
			accessory: false,
		}
	}

	pub fn roof_pitch(xz: Aabb2d, y0: f32, rise: f32, along_x: bool) -> Self {
		Self {
			kind: MassingKind::RoofPitch { along_x },
			xz,
			y0,
			height: rise.max(MIN_EDGE),
			material: None,
			roof: None,
			accessory: false,
		}
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = Some(material);
		self
	}

	pub fn with_material_opt(self, material: Option<MaterialRef>) -> Self {
		match material {
			Some(material) => self.with_material(material),
			None => self,
		}
	}

	pub fn with_roof(mut self, roof: MassingRoof) -> Self {
		self.roof = Some(roof);
		self
	}

	pub fn accessory(mut self) -> Self {
		self.accessory = true;
		self
	}

	pub fn is_roof_only(&self) -> bool {
		matches!(self.kind, MassingKind::RoofPitch { .. })
	}

	pub fn aabb_xz(&self) -> Aabb2d {
		self.xz
	}

	/// Wall / primitive plus optional roof child. UltraLow drops accessories and pitches.
	pub fn drawn_at(&self, ultralow: bool) -> Vec<Self> {
		if ultralow && (self.accessory || self.is_roof_only()) {
			return Vec::new();
		}
		let mut wall = self.clone();
		wall.roof = None;
		let mut out = vec![wall];
		if !ultralow {
			if let Some(roof) = &self.roof {
				out.push(
					Self::roof_pitch(self.xz, self.y0 + self.height, roof.rise, roof.along_x)
						.with_material_opt(roof.material.clone()),
				);
			}
		}
		out
	}
}

/// Four gallery / curtain strips between `outer` and `inner` (full extents).
///
/// `center` is XZ (`y` = world \(z\)). Degenerate when `inner` ≥ `outer`.
pub fn ring_strip_xz(center: Vec2, outer: Vec2, inner: Vec2) -> Vec<Aabb2d> {
	let ox = outer * 0.5;
	let ix = inner.min(outer) * 0.5;
	if ox.x <= ix.x + 1e-3 || ox.y <= ix.y + 1e-3 {
		return vec![Aabb2d { min: center - ox, max: center + ox }];
	}
	vec![
		Aabb2d {
			min: Vec2::new(center.x - ox.x, center.y - ox.y),
			max: Vec2::new(center.x + ox.x, center.y - ix.y),
		},
		Aabb2d {
			min: Vec2::new(center.x - ox.x, center.y + ix.y),
			max: Vec2::new(center.x + ox.x, center.y + ox.y),
		},
		Aabb2d {
			min: Vec2::new(center.x - ox.x, center.y - ix.y),
			max: Vec2::new(center.x - ix.x, center.y + ix.y),
		},
		Aabb2d {
			min: Vec2::new(center.x + ix.x, center.y - ix.y),
			max: Vec2::new(center.x + ox.x, center.y + ix.y),
		},
	]
}

/// Shared unit meshes (must be initialized by [`MassingSilhouettePlugin`]).
#[derive(Resource, Debug, Clone)]
pub struct MassingSilhouetteAssets {
	pub cuboid: Handle<Mesh>,
	pub cylinder: Handle<Mesh>,
	pub frustum: Handle<Mesh>,
	pub roof: Handle<Mesh>,
}

static CUBOID: OnceLock<Handle<Mesh>> = OnceLock::new();
static CYLINDER: OnceLock<Handle<Mesh>> = OnceLock::new();
static FRUSTUM: OnceLock<Handle<Mesh>> = OnceLock::new();
static ROOF: OnceLock<Handle<Mesh>> = OnceLock::new();

impl MassingSilhouetteAssets {
	fn cuboid() -> Handle<Mesh> {
		CUBOID
			.get()
			.expect("MassingSilhouettePlugin must run before massing scenes")
			.clone()
	}

	fn cylinder() -> Handle<Mesh> {
		CYLINDER
			.get()
			.expect("MassingSilhouettePlugin must run before massing scenes")
			.clone()
	}

	fn frustum() -> Handle<Mesh> {
		FRUSTUM
			.get()
			.expect("MassingSilhouettePlugin must run before massing scenes")
			.clone()
	}

	fn roof() -> Handle<Mesh> {
		ROOF.get()
			.expect("MassingSilhouettePlugin must run before massing scenes")
			.clone()
	}

	fn init(meshes: &mut Assets<Mesh>) {
		let _ = CUBOID.set(meshes.add(Mesh::from(Cuboid::from_length(1.0))));
		let _ = CYLINDER.set(meshes.add(Mesh::from(Cylinder { radius: 0.5, half_height: 0.5 })));
		let _ = FRUSTUM.set(meshes.add(unit_frustum_mesh(0.5)));
		let _ = ROOF.set(meshes.add(unit_roof_prism_mesh()));
	}
}

/// Named bands that replace nested kit hosts with primitive massing.
pub fn is_massing_level(level: LodSceneLevel) -> bool {
	matches!(
		level,
		LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_)
	)
}

pub fn is_ultralow_massing(level: LodSceneLevel) -> bool {
	matches!(
		level,
		LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_)
	)
}

/// Unit-mesh transform: translation is the volume center; scale is full edge lengths.
pub fn massing_box_transform(rect: &Aabb2d, y0: f32, height: f32) -> Transform {
	let sx = (rect.max.x - rect.min.x).max(MIN_EDGE);
	let sz = (rect.max.y - rect.min.y).max(MIN_EDGE);
	let sy = height.max(MIN_EDGE);
	Transform {
		translation: Vec3::new(
			(rect.min.x + rect.max.x) * 0.5,
			y0 + sy * 0.5,
			(rect.min.y + rect.max.y) * 0.5,
		),
		scale: Vec3::new(sx, sy, sz),
		..Transform::default()
	}
}

fn volume_transform(volume: &MassingVolume) -> Transform {
	let mut tf = massing_box_transform(&volume.xz, volume.y0, volume.height);
	if let MassingKind::RoofPitch { along_x: false } = volume.kind {
		tf.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
		std::mem::swap(&mut tf.scale.x, &mut tf.scale.z);
	}
	tf
}

fn volume_mesh(volume: &MassingVolume) -> Handle<Mesh> {
	match volume.kind {
		MassingKind::Cuboid => MassingSilhouetteAssets::cuboid(),
		MassingKind::Cylinder => MassingSilhouetteAssets::cylinder(),
		MassingKind::Trazaloid { .. } => MassingSilhouetteAssets::frustum(),
		MassingKind::RoofPitch { .. } => MassingSilhouetteAssets::roof(),
	}
}

fn volume_material(volume: &MassingVolume) -> MaterialRef {
	volume.material.clone().unwrap_or_else(|| {
		if volume.is_roof_only() {
			MaterialRef::named(DEFAULT_ROOF)
		} else {
			MaterialRef::named(DEFAULT_WALL)
		}
	})
}

/// Posed massing primitive with urban [`MaterialRef`] fulfillment.
pub fn massing_primitive_scene(volume: &MassingVolume) -> impl Scene + 'static {
	let mesh = volume_mesh(volume);
	let transform = volume_transform(volume);
	let material = volume_material(volume);
	bsn! {
		Mesh3d({mesh})
		NotShadowCaster
		template_value(transform)
		Visibility::Inherited
		template_value(MaterialRefRoot(material))
		PropagateToDescendants
		LodLazyPending
	}
}

/// Primitive volumes for a far structural band.
pub fn massing_scene(
	building: &impl BuildingComponents,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let Some(probe) = building.structural_lod() else {
		return scene_children(Vec::new());
	};
	let children: Vec<Box<dyn Scene>> = probe
		.massing_volumes(level)
		.iter()
		.map(|volume| Box::new(massing_primitive_scene(volume)) as Box<dyn Scene>)
		.collect();
	scene_children(children)
}

/// Unit frustum: base \([-0.5, 0.5]^2\) at \(y=-0.5\), top scaled by `top_frac` at \(y=0.5\).
fn unit_frustum_mesh(top_frac: f32) -> Mesh {
	let t = (top_frac.clamp(0.08, 1.0)) * 0.5;
	let b = 0.5;
	let y0 = -0.5;
	let y1 = 0.5;
	let positions = vec![
		[-b, y0, -b],
		[b, y0, -b],
		[b, y0, b],
		[-b, y0, b],
		[-t, y1, -t],
		[t, y1, -t],
		[t, y1, t],
		[-t, y1, t],
	];
	let indices = Indices::U16(vec![
		0, 1, 2, 0, 2, 3, // bottom (CCW from -Y)
		4, 6, 5, 4, 7, 6, // top
		0, 5, 1, 0, 4, 5, // -Z
		1, 6, 2, 1, 5, 6, // +X
		2, 7, 3, 2, 6, 7, // +Z
		3, 4, 0, 3, 7, 4, // -X
	]);
	mesh_from_positions(positions, indices)
}

/// Unit roof prism: base at \(y=-0.5\), ridge along X at \(y=0.5, z=0\).
///
/// Centered like the cuboid so [`massing_box_transform`] places the eaves at `y0`
/// and the ridge at `y0 + height`.
fn unit_roof_prism_mesh() -> Mesh {
	let positions = vec![
		[-0.5, -0.5, -0.5],
		[0.5, -0.5, -0.5],
		[0.5, -0.5, 0.5],
		[-0.5, -0.5, 0.5],
		[-0.5, 0.5, 0.0],
		[0.5, 0.5, 0.0],
	];
	let indices = Indices::U16(vec![
		0, 5, 1, 0, 4, 5, // -Z pitch (CCW from outside)
		3, 5, 4, 3, 2, 5, // +Z pitch
		0, 3, 4, // -X gable
		1, 5, 2, // +X gable
	]);
	mesh_from_positions(positions, indices)
}

fn mesh_from_positions(positions: Vec<[f32; 3]>, indices: Indices) -> Mesh {
	let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
	mesh.insert_indices(indices);
	mesh.compute_normals();
	mesh
}

pub struct MassingSilhouettePlugin;

impl Plugin for MassingSilhouettePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init_massing_silhouettes);
	}
}

fn init_massing_silhouettes(mut meshes: ResMut<Assets<Mesh>>) {
	MassingSilhouetteAssets::init(&mut meshes);
}

/// Cuboid massing helper for callers that already pose via [`massing_box_transform`].
pub fn massing_box_scene(transform: Transform) -> impl Scene + 'static {
	let mesh = MassingSilhouetteAssets::cuboid();
	let material = MaterialRef::named(DEFAULT_WALL);
	bsn! {
		Mesh3d({mesh})
		NotShadowCaster
		template_value(transform)
		Visibility::Inherited
		template_value(MaterialRefRoot(material))
		PropagateToDescendants
		LodLazyPending
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn box_transform_centers_and_scales() {
		let rect = Aabb2d { min: Vec2::new(-4.0, -2.0), max: Vec2::new(4.0, 2.0) };
		let tf = massing_box_transform(&rect, 1.0, 6.0);
		assert!((tf.translation - Vec3::new(0.0, 4.0, 0.0)).length() < 1e-4);
		assert!((tf.scale - Vec3::new(8.0, 6.0, 4.0)).length() < 1e-4);
	}

	#[test]
	fn roof_along_z_swaps_plan_scale() {
		let volume = MassingVolume::roof_pitch(
			Aabb2d { min: Vec2::new(-4.0, -1.0), max: Vec2::new(4.0, 1.0) },
			3.0,
			1.5,
			false,
		);
		let tf = volume_transform(&volume);
		assert!((tf.translation.y - 3.75).abs() < 1e-4);
		assert!((tf.scale.x - 2.0).abs() < 1e-4);
		assert!((tf.scale.z - 8.0).abs() < 1e-4);
	}

	#[test]
	fn massing_levels() {
		assert!(is_massing_level(LodSceneLevel::Low));
		assert!(is_massing_level(LodSceneLevel::UltraLow));
		assert!(!is_massing_level(LodSceneLevel::High));
		assert!(!is_massing_level(LodSceneLevel::Medium));
	}

	#[test]
	fn ring_strips_leave_the_courtyard() {
		let strips = ring_strip_xz(Vec2::ZERO, Vec2::new(20.0, 16.0), Vec2::new(10.0, 6.0));
		assert_eq!(strips.len(), 4);
		let court = Aabb2d { min: Vec2::new(-5.0, -3.0), max: Vec2::new(5.0, 3.0) };
		for strip in &strips {
			let overlaps_x = strip.min.x < court.max.x && strip.max.x > court.min.x;
			let overlaps_z = strip.min.y < court.max.y && strip.max.y > court.min.y;
			assert!(!(overlaps_x && overlaps_z), "strip {:?} covers courtyard", strip);
		}
	}
}
