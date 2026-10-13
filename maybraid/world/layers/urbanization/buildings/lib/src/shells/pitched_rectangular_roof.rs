//! A pitched roof shell built from two [`RoofHalf`]s.
//!
//! Each half authors a ridge / eave / wall plate. The main pitch is a
//! [`ClippedRuledStrip`](crate::paneling::clipped_ruled_strip::ClippedRuledStrip);
//! optional wall strip, half-gable walling, and half-hip facets fill the ends.
//! Guiding case: a rectangular hip where both halves share one ridge and eaves
//! run parallel at equal offset. Half-hips bank from the eave end along the
//! eave-perpendicular (local Z) to the ridge plane.
//!
//! **Openings:** `Passage` / `Aperture` assign to the nearest pitch half or drawn
//! gable end; the largest per face wins, clips that face, and maps contact.

mod geometry;
mod openings;

#[cfg(test)]
mod tests;

use bevy_math::bounding::Aabb2d;
use bevy_math::{Vec2, Vec3};
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers, MassingVolume};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;

use crate::openings::{MappedOpenings, Openings};
use crate::paneling::clipped_ruled_strip::ClippedRuledStrip;
use crate::paneling::clipped_tessellated_triangle::ClippedTessellatedTriangle;
use crate::paneling::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, DEFAULT_PANEL_THICKNESS,
};
use crate::paneling::tessellated_triangle_panel::TessellatedTrianglePanel;

use geometry::ResolvedRoofHalf;
pub use geometry::RoofHalf;

/// Authored parameters / builder for a [`PitchedRoof`] shell.
#[derive(Debug, Clone, PartialEq)]
pub struct PitchedRoofParams {
	pub halves: [RoofHalf; 2],
	/// World-space void plan applied at construct time.
	///
	/// **Pitches / gables:** `Passage` and `Aperture` openings are assigned to the
	/// nearest pitch half or drawn gable end; the largest extent on each face
	/// wins and becomes a centered clip.
	pub openings: Openings,
	pub style: PanelStyle,
	pub joint_thickness: f32,
}

impl Default for PitchedRoofParams {
	fn default() -> Self {
		Self::rectangular_hip(Vec2::new(10.0, 6.0), 4.0, 2.5, 1.5)
	}
}

impl PitchedRoofParams {
	pub fn new(halves: [RoofHalf; 2]) -> Self {
		Self {
			halves,
			openings: Openings::new(),
			style: PanelStyle::ShepherdsThatch,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
		}
	}

	/// Axis-aligned rectangular hip about the origin.
	///
	/// `footprint.x` is length along the ridge/eaves (X); `footprint.y` is the
	/// full span between eaves (Z). Ridge is centered on X at `z = 0`, shortened
	/// by `ridge_inset` on each end. Wall plates sit `overhang` inward from each
	/// eave (default overhang 0.3). Both halves draw hips on both ends.
	pub fn rectangular_hip(
		footprint: Vec2,
		ridge_height: f32,
		eave_height: f32,
		ridge_inset: f32,
	) -> Self {
		Self::axis_aligned(footprint, ridge_height, eave_height, ridge_inset, true, false)
	}

	/// Axis-aligned open gable: full-length ridge, gable end walling, no hips.
	pub fn rectangular_gable(footprint: Vec2, ridge_height: f32, eave_height: f32) -> Self {
		Self::axis_aligned(footprint, ridge_height, eave_height, 0.0, false, true)
	}

	fn axis_aligned(
		footprint: Vec2,
		ridge_height: f32,
		eave_height: f32,
		ridge_inset: f32,
		hips: bool,
		gables: bool,
	) -> Self {
		const OVERHANG: f32 = 0.3;
		let half_x = footprint.x * 0.5;
		let half_z = footprint.y * 0.5;
		let ridge_half = (half_x - ridge_inset).max(0.0);
		let wall_z = (half_z - OVERHANG).max(0.0);

		let ridge =
			(Vec3::new(-ridge_half, ridge_height, 0.0), Vec3::new(ridge_half, ridge_height, 0.0));
		let eave_pos =
			(Vec3::new(-half_x, eave_height, half_z), Vec3::new(half_x, eave_height, half_z));
		let eave_neg =
			(Vec3::new(-half_x, eave_height, -half_z), Vec3::new(half_x, eave_height, -half_z));
		let wall_pos =
			(Vec3::new(-half_x, eave_height, wall_z), Vec3::new(half_x, eave_height, wall_z));
		let wall_neg =
			(Vec3::new(-half_x, eave_height, -wall_z), Vec3::new(half_x, eave_height, -wall_z));

		let ends = (true, true);
		let pos = RoofHalf::new(ridge, eave_pos, wall_pos)
			.draw_in_wall_line(true)
			.draw_in_half_hip(if hips { ends } else { (false, false) })
			.draw_in_half_gable_end(if gables { ends } else { (false, false) });
		let neg = RoofHalf::new(ridge, eave_neg, wall_neg)
			.draw_in_wall_line(true)
			.draw_in_half_hip(if hips { ends } else { (false, false) })
			.draw_in_half_gable_end(if gables { ends } else { (false, false) });

		Self::new([pos, neg])
	}

	pub fn openings(mut self, openings: Openings) -> Self {
		self.openings = openings;
		self
	}

	pub fn style(mut self, style: PanelStyle) -> Self {
		self.style = style;
		self
	}

	pub fn joint_thickness(mut self, joint_thickness: f32) -> Self {
		self.joint_thickness = joint_thickness;
		self
	}

	pub fn build(self) -> PitchedRoof {
		PitchedRoof::new(self)
	}
}

/// Two-half pitched roof shell.
#[derive(Debug, Clone, PartialEq)]
pub struct PitchedRoof {
	params: PitchedRoofParams,
	joint_policy: PanelComplexJointPolicy,
	halves: [ResolvedRoofHalf; 2],
	/// Winning openings (at most one per pitch half / gable end).
	openings: Openings,
	/// Contact geometry for those openings.
	mapped: MappedOpenings,
	surface_material: Option<MaterialRef>,
}

impl PitchedRoof {
	pub fn new(params: PitchedRoofParams) -> Self {
		let joint_policy = PanelComplexJointPolicy::default();
		let style = params.style;

		let resolved = params.resolve_roof_openings();
		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		let mut pitch_clips: [Option<Vec<Vec3>>; 2] = [None, None];
		let mut gable_clips: [Option<Vec<Vec3>>; 2] = [None, None];
		for entry in resolved.pitch.into_iter().flatten() {
			pitch_clips[entry.half] = Some(entry.clip);
			mapped.insert(entry.id.clone(), entry.mapped);
			openings.insert(entry.id, entry.opening);
		}
		for entry in resolved.gable.into_iter().flatten() {
			gable_clips[entry.end] = Some(entry.clip);
			mapped.insert(entry.id.clone(), entry.mapped);
			openings.insert(entry.id, entry.opening);
		}

		let halves = [
			params.halves[0].resolve(
				style,
				joint_policy,
				pitch_clips[0].clone(),
				gable_clips.clone(),
			),
			params.halves[1].resolve(style, joint_policy, pitch_clips[1].clone(), gable_clips),
		];
		Self { params, joint_policy, halves, openings, mapped, surface_material: None }
	}

	/// Stamp a roof shader look onto every emitted panel (kit style unchanged).
	pub fn with_surface_material(mut self, material: MaterialRef) -> Self {
		self.surface_material = Some(material);
		self
	}

	pub fn surface_material(&self) -> Option<&MaterialRef> {
		self.surface_material.as_ref()
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		let resolved = self.params.resolve_roof_openings();
		let mut pitch_clips: [Option<Vec<Vec3>>; 2] = [None, None];
		let mut gable_clips: [Option<Vec<Vec3>>; 2] = [None, None];
		for entry in resolved.pitch.into_iter().flatten() {
			pitch_clips[entry.half] = Some(entry.clip);
		}
		for entry in resolved.gable.into_iter().flatten() {
			gable_clips[entry.end] = Some(entry.clip);
		}
		self.halves = [
			self.params.halves[0].resolve(
				self.params.style,
				joint_policy,
				pitch_clips[0].clone(),
				gable_clips.clone(),
			),
			self.params.halves[1].resolve(
				self.params.style,
				joint_policy,
				pitch_clips[1].clone(),
				gable_clips,
			),
		];
		self
	}

	pub fn params(&self) -> &PitchedRoofParams {
		&self.params
	}

	pub fn pitches(&self) -> [&ClippedRuledStrip; 2] {
		[&self.halves[0].pitch, &self.halves[1].pitch]
	}

	pub fn wall_complexes(&self) -> [Option<&PanelComplex>; 2] {
		[self.halves[0].wall.as_ref(), self.halves[1].wall.as_ref()]
	}

	pub fn hip_panels(&self) -> impl Iterator<Item = &TessellatedTrianglePanel> {
		self.halves.iter().flat_map(|h| h.hips.iter())
	}

	pub fn gable_panels(&self) -> impl Iterator<Item = &ClippedTessellatedTriangle> {
		self.halves.iter().flat_map(|h| h.gables.iter())
	}

	pub fn hip_count(&self) -> usize {
		self.halves.iter().map(|h| h.hips.len()).sum()
	}

	pub fn gable_count(&self) -> usize {
		self.halves.iter().map(|h| h.gables.len()).sum()
	}
}

impl BuildingComponents for PitchedRoof {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for half in &self.halves {
			out.extend(half.pitch.panel_nodes_for_level(level));
			if let Some(wall) = &half.wall {
				out.extend(wall.panel_nodes_for_level(level));
			}
			for hip in &half.hips {
				out.extend(hip.panel_nodes_for_level(level));
			}
			for gable in &half.gables {
				out.extend(gable.as_complex().panel_nodes_for_level(level));
			}
		}
		if let Some(material) = &self.surface_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for half in &self.halves {
			out.extend(half.pitch.joint_nodes_for_level(level));
			if let Some(wall) = &half.wall {
				out.extend(wall.joint_nodes_for_level(level));
			}
			for gable in &half.gables {
				out.extend(gable.as_complex().joint_nodes_for_level(level));
			}
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = Vec3::splat(f32::NEG_INFINITY);
		for half in &self.params.halves {
			for p in [
				half.ridge_line.0,
				half.ridge_line.1,
				half.eave_line.0,
				half.eave_line.1,
				half.wall_line.0,
				half.wall_line.1,
			] {
				min = min.min(p);
				max = max.max(p);
			}
		}
		if !min.x.is_finite() {
			return None;
		}
		let ridge = self.params.halves[0].ridge_line;
		let along_x = (ridge.1.x - ridge.0.x).abs() >= (ridge.1.z - ridge.0.z).abs();
		let xz = Aabb2d { min: Vec2::new(min.x, min.z), max: Vec2::new(max.x, max.z) };
		Some(BuildingStructuralLodProbe::from_volumes([MassingVolume::roof_pitch(
			xz,
			min.y,
			(max.y - min.y).max(0.3),
			along_x,
		)
		.with_material_opt(self.surface_material.clone())]))
	}
}
