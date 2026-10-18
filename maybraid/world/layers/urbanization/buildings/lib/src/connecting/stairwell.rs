//! Exclusive [`WellAabb`] stairwell: run-in + circular, rectangular, or
//! run-and-landing flight + exit landing.
//!
//! Two horizontal shaft faces allocate one orthogonal box. Offset / skew / size
//! mismatch is another well (or a hall), not a polyline. Walk-off is a landing.
//! The last tread arrives at that landing's interior edge (the back-point).
//! Circuit wrapping stays at one lap (a tight going is accepted). Run-and-landing
//! may add one routing switchback so a leftover U is not required.

mod laws;
mod opening;
mod rect;
mod run_and_landing;
mod spiral;
mod tread;
mod well;

pub use opening::StairwellOpening;
pub use tread::TreadEnd;
pub use well::{WellAabb, WellSide};

use bevy_math::Vec3;
use building_components::floors::FloorNode;
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::stairs::StairNode;
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;

use crate::paneling::panel_complex::{PanelComplexJointPolicy, DEFAULT_PANEL_THICKNESS};
use crate::paneling::quad_panel::QuadPanel;
use crate::paneling::rectangle::Rectangle;

/// Aesthetic run-in depth along the walk-on (meters).
pub const RUN_IN_M: f32 = 0.75;

/// Kit thickness for owned floor slabs (meters).
pub const SLAB_THICKNESS_M: f32 = 0.05;

/// Default tread span as a fraction of the well's tighter half-extent.
pub const TREAD_FILL_DEFAULT: f32 = well::TREAD_FILL_DEFAULT;

/// Circular helix, wall-hugging rectangular flights, or half-well I + landing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StairwellKind {
	#[default]
	Circular,
	Rectangular,
	/// Half-well I flights; L/U are walkable-depth strips to the walk-off.
	/// Extra laps turn around on an interior landing and run the other half.
	/// A leftover U tries one routing switchback first.
	RunAndLanding,
}

/// Exclusive box → run-in + flight + optional walk-off landing.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectingStairwell {
	style: PanelStyle,
	well: WellAabb,
	kind: StairwellKind,
	run_in: QuadPanel,
	want_landing: bool,
	slab_thickness: f32,
	upper_landing: Option<QuadPanel>,
	mid_landings: Vec<QuadPanel>,
	stairs: Vec<StairNode>,
	/// Rectangular liners on faces that are not walk-on or walk-off.
	want_shaft_walls: bool,
	shaft_walls: Vec<Rectangle>,
	surface_material: Option<MaterialRef>,
	stair_material: Option<MaterialRef>,
}

impl ConnectingStairwell {
	/// Allocate an exclusive box from two shaft faces. `lower` is the floor-space
	/// anchor even when both faces share a \(Y\).
	pub fn new(
		style: PanelStyle,
		lower: impl Into<StairwellOpening>,
		upper: impl Into<StairwellOpening>,
	) -> Self {
		Self::from_well(style, WellAabb::allocate(lower.into(), upper.into(), TREAD_FILL_DEFAULT))
	}

	/// Fit a circular spiral into an already-allocated exclusive box.
	pub fn from_well(style: PanelStyle, well: WellAabb) -> Self {
		Self::from_well_kind(style, well, StairwellKind::Circular)
	}

	/// Fit [`StairwellKind`] into an already-allocated exclusive box.
	pub fn from_well_kind(style: PanelStyle, well: WellAabb, kind: StairwellKind) -> Self {
		let slab_thickness = SLAB_THICKNESS_M;
		let Fit { stairs, door, mids } = fit_kind(&well, style, slab_thickness, kind, true);
		Self {
			style,
			kind,
			run_in: well.run_in_slab(style, slab_thickness),
			want_landing: true,
			slab_thickness,
			upper_landing: door,
			mid_landings: mids,
			stairs,
			well,
			want_shaft_walls: false,
			shaft_walls: Vec::new(),
			surface_material: None,
			stair_material: None,
		}
	}

	pub fn rough_stone(
		lower: impl Into<StairwellOpening>,
		upper: impl Into<StairwellOpening>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, lower, upper)
	}

	/// Walk-off landing. Default `true`. Set `false` when a follow-on well owns
	/// that floor (its run-in).
	pub fn with_upper_landing(mut self, enabled: bool) -> Self {
		self.want_landing = enabled;
		self.rebuild();
		self
	}

	/// Rectangular panels on well faces that are not walk-on or walk-off.
	///
	/// Default `false`. Keep / tower shafts turn this on so the hole is lined
	/// on the three closed sides.
	pub fn with_shaft_walls(mut self, enabled: bool) -> Self {
		self.want_shaft_walls = enabled;
		self.rebuild();
		self
	}

	/// Kit thickness of owned slabs (meters). Default [`SLAB_THICKNESS_M`].
	pub fn with_slab_thickness(mut self, thickness: f32) -> Self {
		self.slab_thickness = thickness.max(1e-4);
		self.rebuild();
		self
	}

	/// Tread span as a fraction of the tighter well half-extent.
	///
	/// Default [`TREAD_FILL_DEFAULT`]. Clamped to \(0.2\ldots 0.95\).
	pub fn with_tread_fill(mut self, fill: f32) -> Self {
		self.well.tread_fill = well::clamp_tread_fill(fill);
		self.rebuild();
		self
	}

	/// Circular, rectangular, or run-and-landing. Default [`StairwellKind::Circular`].
	pub fn with_kind(mut self, kind: StairwellKind) -> Self {
		self.kind = kind;
		self.rebuild();
		self
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.run_in = self.run_in.clone().with_joint_policy(joint_policy);
		self.upper_landing =
			self.upper_landing.take().map(|slab| slab.with_joint_policy(joint_policy));
		self.mid_landings = self
			.mid_landings
			.drain(..)
			.map(|slab| slab.with_joint_policy(joint_policy))
			.collect();
		self
	}

	/// Stamp a wall shader look onto stairwell panels (kit style unchanged).
	pub fn with_surface_material(mut self, material: MaterialRef) -> Self {
		self.surface_material = Some(material);
		self
	}

	/// Stamp a shader look onto treads and landings. Falls back to [`Self::surface_material`].
	pub fn with_stair_material(mut self, material: MaterialRef) -> Self {
		self.stair_material = Some(material);
		self
	}

	pub fn surface_material(&self) -> Option<&MaterialRef> {
		self.surface_material.as_ref()
	}

	pub fn stair_material(&self) -> Option<&MaterialRef> {
		self.stair_material.as_ref().or(self.surface_material.as_ref())
	}

	fn rebuild(&mut self) {
		let Fit { stairs, door, mids } =
			fit_kind(&self.well, self.style, self.slab_thickness, self.kind, self.want_landing);
		self.stairs = stairs;
		self.run_in = self.well.run_in_slab(self.style, self.slab_thickness);
		self.upper_landing = door;
		self.mid_landings = mids;
		self.shaft_walls = if self.want_shaft_walls {
			enclosure_walls(&self.well, self.style)
		} else {
			Vec::new()
		};
	}

	pub fn well(&self) -> WellAabb {
		self.well
	}

	pub fn kind(&self) -> StairwellKind {
		self.kind
	}

	pub fn slab_thickness(&self) -> f32 {
		self.slab_thickness
	}

	pub fn tread_fill(&self) -> f32 {
		self.well.tread_fill
	}

	pub fn run_in(&self) -> &QuadPanel {
		&self.run_in
	}

	pub fn upper_landing(&self) -> Option<&QuadPanel> {
		self.upper_landing.as_ref()
	}

	pub fn mid_landings(&self) -> &[QuadPanel] {
		&self.mid_landings
	}

	pub fn shaft_walls(&self) -> &[Rectangle] {
		&self.shaft_walls
	}

	pub fn stairs(&self) -> &[StairNode] {
		&self.stairs
	}

	pub fn last_tread_end(&self) -> Option<TreadEnd> {
		TreadEnd::from_last_straight(&self.stairs)
	}
}

impl BuildingComponents for ConnectingStairwell {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut landings = self.run_in.panel_nodes_for_level(level);
		for pad in &self.mid_landings {
			landings.extend(pad.panel_nodes_for_level(level));
		}
		if let Some(landing) = &self.upper_landing {
			landings.extend(landing.panel_nodes_for_level(level));
		}
		if let Some(material) = self.stair_material() {
			landings = landings.with_material(material.clone());
		}
		let mut walls = Layers::new();
		for wall in &self.shaft_walls {
			walls.extend(wall.panel_nodes_for_level(level));
		}
		if let Some(material) = &self.surface_material {
			walls = walls.with_material(material.clone());
		}
		landings.extend(walls);
		landings
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.run_in.joint_nodes_for_level(level);
		for pad in &self.mid_landings {
			out.extend(pad.joint_nodes_for_level(level));
		}
		if let Some(landing) = &self.upper_landing {
			out.extend(landing.joint_nodes_for_level(level));
		}
		out
	}

	fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
		Layers::new()
	}

	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		let mut out = Layers::from_free(self.stairs.clone());
		if let Some(material) = self.stair_material() {
			out = out.with_material(material.clone());
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let mut probe = BuildingStructuralLodProbe::from_aabb3d_xz(
			bevy_math::Vec3::from(self.well.bounds.min),
			bevy_math::Vec3::from(self.well.bounds.max),
		);
		for volume in &mut probe.volumes {
			volume.accessory = true;
			if let Some(material) = &self.surface_material {
				volume.material = Some(material.clone());
			}
		}
		Some(probe)
	}
}

pub(crate) struct Fit {
	pub(crate) stairs: Vec<StairNode>,
	pub(crate) door: Option<QuadPanel>,
	pub(crate) mids: Vec<QuadPanel>,
}

impl Fit {
	fn with_door(self, want: bool) -> Self {
		Self { door: want.then_some(self.door).flatten(), ..self }
	}
}

/// Closed well faces as ordinary rectangle kits, thickness toward the well.
fn enclosure_walls(well: &WellAabb, style: PanelStyle) -> Vec<Rectangle> {
	[WellSide::NegZ, WellSide::PosX, WellSide::PosZ, WellSide::NegX]
		.into_iter()
		.filter(|side| *side != well.walk_on && *side != well.walk_off)
		.map(|side| {
			let mid = well.side_mid(side, well.bottom_y());
			let along = side.travel_xz();
			let half = well.face_half(side);
			let origin = Vec3::new(mid.x - along.x * half, well.bottom_y(), mid.z - along.y * half);
			let edge = Vec3::new(along.x * 2.0 * half, 0.0, along.y * 2.0 * half);
			Rectangle::new(style, origin, edge, well.rise(), DEFAULT_PANEL_THICKNESS, 0.0)
		})
		.collect()
}

fn fit_kind(
	well: &WellAabb,
	style: PanelStyle,
	thickness: f32,
	kind: StairwellKind,
	want_landing: bool,
) -> Fit {
	match kind {
		StairwellKind::Circular => spiral::fit(well, style, thickness).with_door(want_landing),
		StairwellKind::Rectangular => rect::fit(well, style, thickness).with_door(want_landing),
		StairwellKind::RunAndLanding => run_and_landing::fit(well, style, thickness, want_landing),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::MappedOpening;
	use bevy_math::{Vec2, Vec3};
	use building_components::normalize_xz;
	use building_components::partitions::PANEL_Y_HALF;
	use building_components::stairs::Stair;

	/// Horizontal shaft face: `center` in the hole, walk-on on the −orientation side.
	fn shaft_opening(
		center: Vec3,
		half_w: f32,
		half_d: f32,
		orient: Vec2,
	) -> anyhow::Result<MappedOpening> {
		let d = normalize_xz(orient)
			.ok_or_else(|| anyhow::anyhow!("orientation too short: {orient:?}"))?;
		let right = Vec3::new(-d.y, 0.0, d.x);
		let out = Vec3::new(d.x, 0.0, d.y);
		let walk = center - out * half_d;
		let far = center + out * half_d;
		let bl = walk - right * half_w;
		let br = walk + right * half_w;
		let tl = far - right * half_w;
		let tr = far + right * half_w;
		Ok(MappedOpening::from_corners(bl, br, tl, tr, orient))
	}

	fn pad_aabb(panel: &QuadPanel) -> (Vec2, Vec2) {
		let [c0, c1, c2, c3] = panel.corners();
		(
			Vec2::new(c0.x.min(c1.x).min(c2.x).min(c3.x), c0.z.min(c1.z).min(c2.z).min(c3.z)),
			Vec2::new(c0.x.max(c1.x).max(c2.x).max(c3.x), c0.z.max(c1.z).max(c2.z).max(c3.z)),
		)
	}

	fn covers(pad: &QuadPanel, p: Vec2) -> bool {
		let (mn, mx) = pad_aabb(pad);
		p.x >= mn.x - 0.04 && p.x <= mx.x + 0.04 && p.y >= mn.y - 0.04 && p.y <= mx.y + 0.04
	}

	#[test]
	fn stacked_shafts_allocate_an_exclusive_box() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		let aabb = well.well();
		assert!((aabb.rise() - 3.0).abs() < 1e-3);
		assert_eq!(aabb.walk_on, WellSide::NegZ);
		assert_eq!(aabb.walk_off, WellSide::NegZ);
		assert!(aabb.half_x() > 1.1 && aabb.half_z() > 1.1);
		Ok(())
	}

	#[test]
	fn run_in_follows_walk_on_into_the_box() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::X)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::X)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		assert!((well.run_in().thickness() - SLAB_THICKNESS_M).abs() < 1e-3);
		let [a0, a1, b0, b1] = well.run_in().corners();
		assert!(
			(a0.y - (-PANEL_Y_HALF)).abs() < 1e-3,
			"run-in kit center should sit {PANEL_Y_HALF} below the walk-on, got y={}",
			a0.y
		);
		let inward = (b0 + b1) * 0.5 - (a0 + a1) * 0.5;
		assert!(inward.x > 0.5, "run-in should follow +X into the shaft, got {inward:?}");
		Ok(())
	}

	#[test]
	fn spiral_stays_in_the_box_and_lands_to_walk_off() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		let stairs = well.stairs();
		assert!(!stairs.is_empty());
		assert!(stairs.iter().all(|s| matches!(s.geometry, Stair::Straight(_))));
		let aabb = well.well();
		for s in stairs {
			let p = s.placement.translation;
			assert!(aabb.contains_xz(p.x, p.z), "spiral treads should sit in the well, got {p:?}");
		}
		let landing = well.upper_landing().expect("walk-off landing");
		let walk_off = aabb.side_mid(aabb.walk_off, aabb.top_y());
		let door = aabb.walk_off.into_xz();
		let on_door = landing.corners().into_iter().any(|p| {
			let along_out = (p.x - walk_off.x) * door.x + (p.z - walk_off.z) * door.y;
			along_out.abs() < 0.04
		});
		assert!(on_door, "landing should sit on the walk-off edge");
		let last = well.last_tread_end().expect("last tread");
		let last_mid = last.leading_mid();
		let last_to_door = (last_mid - Vec2::new(walk_off.x, walk_off.z)).length();
		assert!(
			last_to_door > 0.15,
			"last tread must not sit on the walk-off, dist={last_to_door}"
		);
		let (pad_min, pad_max) = pad_aabb(landing);
		for (label, p) in [("outer", last.leading_outer), ("inner", last.leading_inner)] {
			assert!(covers(landing, p), "landing must cover last leading {label} {p:?}");
		}
		let along = match aabb.walk_off {
			WellSide::NegZ | WellSide::PosZ => pad_max.x - pad_min.x,
			WellSide::NegX | WellSide::PosX => pad_max.y - pad_min.y,
		};
		let well_along = 2.0 * aabb.face_half(aabb.walk_off);
		assert!(
			(along - well_along).abs() < 0.04,
			"landing along-wall {along} should span the well face {well_along}"
		);
		Ok(())
	}

	#[test]
	fn quarter_turn_landing_is_a_strip_not_a_lid() {
		let well = ConnectingStairwell::from_well(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegX,
				TREAD_FILL_DEFAULT,
			),
		);
		let aabb = well.well();
		let landing = well.upper_landing().expect("walk-off landing");
		let (pad_min, pad_max) = pad_aabb(landing);
		let inward = pad_max.x - pad_min.x;
		assert!(
			inward < aabb.half_x() + 0.04,
			"quarter-turn landing must not lid the well, inward={inward}"
		);
		let center = aabb.center_xz();
		assert!(
			center.x < pad_min.x - 0.04 || center.x > pad_max.x + 0.04,
			"well center {center:?} should stay off the landing {pad_min:?}..{pad_max:?}"
		);
		let last = well.last_tread_end().expect("last tread");
		let door = aabb.side_mid(aabb.walk_off, aabb.top_y());
		let last_to_door = (last.leading_mid() - Vec2::new(door.x, door.z)).length();
		assert!(
			last_to_door < aabb.half_min(),
			"last tread should arrive at the walk-off strip, dist={last_to_door}"
		);
	}

	#[test]
	fn omitting_upper_landing_leaves_only_run_in() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let with = ConnectingStairwell::rough_stone(lower, upper);
		let without = ConnectingStairwell::rough_stone(lower, upper).with_upper_landing(false);
		assert!(with.upper_landing().is_some());
		assert!(without.upper_landing().is_none());
		Ok(())
	}

	#[test]
	fn tread_fill_widens_treads() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let default = ConnectingStairwell::rough_stone(lower, upper);
		let wide = ConnectingStairwell::rough_stone(lower, upper).with_tread_fill(0.8);
		assert!((default.tread_fill() - TREAD_FILL_DEFAULT).abs() < 1e-4);
		assert!((wide.tread_fill() - 0.8).abs() < 1e-4);
		let w0 = first_tread_width(&default);
		let w1 = first_tread_width(&wide);
		assert!(w1 > w0 + 0.2, "fill 0.8 should be wider than default, {w0} vs {w1}");
		Ok(())
	}

	#[test]
	fn tall_well_keeps_one_circuit() {
		let short = WellAabb::from_plan(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
			TREAD_FILL_DEFAULT,
		);
		let tall = WellAabb::from_plan(
			Vec3::new(-1.7, 0.0, -1.7),
			Vec3::new(1.7, 4.8, 1.7),
			WellSide::PosX,
			WellSide::NegX,
			0.55,
		);
		let a = ConnectingStairwell::from_well(PanelStyle::RoughStonework, short);
		let b = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			tall,
			StairwellKind::Rectangular,
		);
		let spiral_turns = |well: &ConnectingStairwell, aabb: &WellAabb| {
			let going = match &well.stairs()[0].geometry {
				Stair::Straight(g) => g.going_per_tread(),
				Stair::Spiral(_) => panic!("expected straight treads"),
			};
			let center = aabb.center_xz();
			let p = well.stairs()[0].placement.translation;
			let radius = (Vec2::new(p.x, p.z) - center).length().max(1e-4);
			let intervals = well.stairs().len().saturating_sub(1).max(1) as f32;
			going * intervals / (std::f32::consts::TAU * radius)
		};
		assert!(spiral_turns(&a, &short) < 1.5, "short same-side spiral must stay one circuit");
		assert_eq!(
			b.stairs().len(),
			2,
			"4.8 m opposite-face keep well should stay one L, not a second lap"
		);
	}

	#[test]
	fn short_well_keeps_one_lap_instead_of_stacking() {
		let tiny = WellAabb::from_plan(
			Vec3::new(-0.6, 0.0, -0.6),
			Vec3::new(0.6, 1.5, 0.6),
			WellSide::NegZ,
			WellSide::NegZ,
			TREAD_FILL_DEFAULT,
		);
		let well = ConnectingStairwell::from_well(PanelStyle::RoughStonework, tiny);
		assert!(!well.stairs().is_empty());
		let going = match &well.stairs()[0].geometry {
			Stair::Straight(g) => g.going_per_tread(),
			Stair::Spiral(_) => panic!("spiral well should emit Straight treads"),
		};
		let center = tiny.center_xz();
		let p = well.stairs()[0].placement.translation;
		let radius = (Vec2::new(p.x, p.z) - center).length().max(1e-4);
		let intervals = well.stairs().len().saturating_sub(1).max(1) as f32;
		let turns = going * intervals / (std::f32::consts::TAU * radius);
		assert!(turns < 1.5, "1.5 m well must not add a second lap, turns={turns} going={going}");
		assert!(
			tiny.rise() / turns.max(1e-4) + 1e-3 >= tiny.rise() - 0.05,
			"one lap should keep the full rise as headroom, turns={turns}"
		);
	}

	#[test]
	fn shaft_walls_omit_enter_and_exit_faces() {
		use building_components::panels::PanelGeometry;

		let same = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 4.8, 1.2),
				WellSide::PosX,
				WellSide::PosX,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		)
		.with_shaft_walls(true);
		assert_eq!(same.shaft_walls().len(), 3, "same-face well leaves three closed sides");
		assert!(same
			.shaft_walls()
			.iter()
			.all(|w| { matches!(w.panel.geometry, PanelGeometry::Rectangle(_)) }));

		let opposite = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 4.8, 1.2),
				WellSide::PosX,
				WellSide::NegX,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		)
		.with_shaft_walls(true);
		assert_eq!(opposite.shaft_walls().len(), 2, "opposite doors leave two closed sides");
		assert!(ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegZ,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		)
		.shaft_walls()
		.is_empty());
	}

	#[test]
	fn stacked_wells_share_the_walk_off_face() {
		let lower = WellAabb::from_plan(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
			TREAD_FILL_DEFAULT,
		);
		let upper = WellAabb::from_plan(
			Vec3::new(-1.2, 3.0, -1.2),
			Vec3::new(1.2, 6.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
			TREAD_FILL_DEFAULT,
		);
		assert_eq!(lower.walk_off, upper.walk_on);
		assert!((lower.top_y() - upper.bottom_y()).abs() < 1e-4);
		let a = ConnectingStairwell::from_well(PanelStyle::RoughStonework, lower)
			.with_upper_landing(false);
		let b = ConnectingStairwell::from_well(PanelStyle::RoughStonework, upper);
		assert!(a.upper_landing().is_none());
		assert!(b.upper_landing().is_some());
		assert!(!a.stairs().is_empty());
		assert!(!b.stairs().is_empty());
	}

	#[test]
	fn rectangular_same_side_hugs_four_walls() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegZ,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		assert_eq!(well.kind(), StairwellKind::Rectangular);
		assert_eq!(well.stairs().len(), 4);
		assert_eq!(well.mid_landings().len(), 3);
		assert!(well.upper_landing().is_some());
		let aabb = well.well();
		for s in well.stairs() {
			let p = s.placement.translation;
			assert!(
				aabb.contains_xz(p.x, p.z),
				"rectangular flight should stay in the well, {p:?}"
			);
		}
	}

	#[test]
	fn rectangular_opposite_is_two_flights() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::PosZ,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		assert_eq!(well.stairs().len(), 2);
		assert_eq!(well.mid_landings().len(), 1);
	}

	#[test]
	fn rectangular_quarter_landing_is_a_strip() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegX,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		let aabb = well.well();
		let landing = well.upper_landing().expect("walk-off");
		let (pad_min, pad_max) = pad_aabb(landing);
		let center = aabb.center_xz();
		assert!(
			center.x < pad_min.x - 0.04 || center.x > pad_max.x + 0.04,
			"rectangular quarter-turn must not lid the well"
		);
		assert!(!well.stairs().is_empty());
	}

	#[test]
	fn rectangular_turn_pad_meets_both_flights() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegZ,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		let stairs = well.stairs();
		assert!(stairs.len() >= 2, "need a turn");
		for i in 0..stairs.len() - 1 {
			let end = TreadEnd::from_straight(&stairs[i]);
			let next = TreadEnd::from_straight(&stairs[i + 1]);
			let origin = Vec2::new(
				stairs[i + 1].placement.translation.x,
				stairs[i + 1].placement.translation.z,
			);
			let first_trail = origin - next.travel * (0.5 * next.going);
			let covered = well
				.mid_landings()
				.iter()
				.any(|pad| covers(pad, end.leading_mid()) && covers(pad, first_trail));
			assert!(
				covered,
				"landing {i} should fill the gap between flights (end {:?}, next start {:?})",
				end.leading_mid(),
				first_trail
			);
		}
	}

	#[test]
	fn rectangular_last_leading_meets_the_end_pad() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegZ,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		let aabb = well.well();
		for (i, node) in well.stairs().iter().enumerate() {
			let end = TreadEnd::from_straight(node);
			let side = [WellSide::NegZ, WellSide::PosX, WellSide::PosZ, WellSide::NegX]
				.into_iter()
				.max_by(|a, b| {
					end.travel
						.dot(a.travel_xz())
						.partial_cmp(&end.travel.dot(b.travel_xz()))
						.unwrap_or(std::cmp::Ordering::Equal)
				})
				.expect("side");
			let want = rect::flight_end_leading(&aabb, side);
			let got = end.leading_mid();
			assert!(
				(got - want).length() < 0.05,
				"flight {i} last leading {got:?} should meet end pad {want:?} on {side:?}"
			);
		}
	}

	#[test]
	fn rectangular_skinny_collapses_the_hole() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-0.35, 0.0, -1.4),
				Vec3::new(0.35, 3.0, 1.4),
				WellSide::NegZ,
				WellSide::NegZ,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		assert!(!well.stairs().is_empty());
		let aabb = well.well();
		for s in well.stairs() {
			let p = s.placement.translation;
			assert!(
				aabb.contains_xz(p.x, p.z),
				"skinny rectangular should still hug the box, {p:?}"
			);
		}
	}

	fn first_tread_width(well: &ConnectingStairwell) -> f32 {
		match &well.stairs()[0].geometry {
			Stair::Straight(g) => g.width,
			Stair::Spiral(_) => panic!("spiral well should emit Straight treads"),
		}
	}

	fn run_well(min: Vec3, max: Vec3, on: WellSide, off: WellSide) -> ConnectingStairwell {
		ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(min, max, on, off, TREAD_FILL_DEFAULT),
			StairwellKind::RunAndLanding,
		)
	}

	#[test]
	fn run_and_landing_fills_half_the_well() {
		let well = run_well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::PosZ,
		);
		assert_eq!(well.kind(), StairwellKind::RunAndLanding);
		assert_eq!(well.stairs().len(), 1);
		let w = first_tread_width(&well);
		assert!((w - 1.2).abs() < 0.04, "I should fill the east half, width={w}");
		let p = well.stairs()[0].placement.translation;
		assert!(p.x > 0.2, "first I hugs the CCW (east) half, x={}", p.x);
		assert!(well.well().contains_xz(p.x, p.z));
	}

	#[test]
	fn run_and_landing_opposite_is_just_the_door() {
		let well = run_well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::PosZ,
		);
		assert!(well.mid_landings().is_empty());
		assert!(well.upper_landing().is_some());
		let last = well.last_tread_end().expect("last tread");
		let door = well.well().side_mid(WellSide::PosZ, well.well().top_y());
		let last_to_door = (last.leading_mid() - Vec2::new(door.x, door.z)).length();
		assert!(last_to_door > 0.08, "last leading must not sit on the door");
		assert!(
			covers(well.upper_landing().unwrap(), last.leading_mid()),
			"door strip should cover last leading"
		);
	}

	#[test]
	fn run_and_landing_same_side_laps_back_once() {
		let well = run_well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
		);
		assert_eq!(well.stairs().len(), 2, "U to walk-off should switchback once first");
		let a = well.stairs()[0].placement.translation;
		let b = well.stairs()[1].placement.translation;
		assert!(a.x > 0.1 && b.x < -0.1, "return I occupies the other half");
		assert_eq!(well.mid_landings().len(), 1, "interior turnaround only");
		let (mn, mx) = pad_aabb(&well.mid_landings()[0]);
		let depth = mx.y - mn.y;
		assert!(
			(0.85..1.05).contains(&depth),
			"interior landing should be the walkable min, got {depth}"
		);
		let turn = TreadEnd::from_straight(&well.stairs()[0]);
		assert!(covers(&well.mid_landings()[0], turn.leading_mid()));
		assert!(well.upper_landing().is_some());
		let last = well.last_tread_end().expect("last tread");
		assert!(covers(well.upper_landing().unwrap(), last.leading_mid()));
	}

	#[test]
	fn run_and_landing_adjacent_is_a_walkable_l() {
		let well = run_well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegX,
		);
		assert_eq!(well.stairs().len(), 1);
		let last = well.last_tread_end().expect("last tread");
		assert!(well.mid_landings().iter().any(|p| covers(p, last.leading_mid())));
		let (mn, mx) = pad_aabb(&well.mid_landings()[0]);
		assert!(
			(0.85..1.05).contains(&(mx.y - mn.y)),
			"L end-leg should be walkable depth, got {:?}",
			pad_aabb(&well.mid_landings()[0])
		);
		let landing = well.upper_landing().expect("walk-off L leg");
		let (dn, dx) = pad_aabb(landing);
		assert!(
			(0.85..1.05).contains(&(dx.x - dn.x)),
			"L door-leg should be walkable depth, got {dn:?}..{dx:?}"
		);
	}

	#[test]
	fn run_and_landing_hug_adjacent_keeps_the_i_open() {
		let well = run_well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::PosX,
		);
		assert_eq!(well.stairs().len(), 1);
		let landing = well.upper_landing().expect("walk-off");
		let (mn, mx) = pad_aabb(landing);
		let along = mx.y - mn.y;
		assert!(
			along < well.well().half_z() + 0.08,
			"hug-wall door must not lid the I, along={along}"
		);
		let (a, b) = pad_aabb(&well.mid_landings()[0]);
		assert!((0.85..1.05).contains(&(b.y - a.y)), "hug L end-leg should be walkable depth");
	}

	#[test]
	fn run_and_landing_tiny_same_side_still_laps_back() {
		let well = run_well(
			Vec3::new(-0.6, 0.0, -0.6),
			Vec3::new(0.6, 1.5, 0.6),
			WellSide::NegZ,
			WellSide::NegZ,
		);
		assert_eq!(well.stairs().len(), 2);
		assert_eq!(well.mid_landings().len(), 1);
	}

	#[test]
	fn run_and_landing_one_tread_uses_walkable_u_strips() {
		let well = run_well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 0.18, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
		);
		assert_eq!(well.stairs().len(), 1);
		assert_eq!(well.mid_landings().len(), 2, "U extras: far + unused side");
		for pad in well.mid_landings() {
			let (mn, mx) = pad_aabb(pad);
			let d = (mx.x - mn.x).min(mx.y - mn.y);
			assert!(
				(0.85..1.05).contains(&d),
				"U legs should be walkable strips, got {mn:?}..{mx:?}"
			);
		}
	}

	#[test]
	fn run_and_landing_tall_laps_run_the_other_half() {
		let well = run_well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 6.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
		);
		assert!(well.stairs().len() >= 2, "6 m should add a return I, got {}", well.stairs().len());
		let a = well.stairs()[0].placement.translation;
		let b = well.stairs()[1].placement.translation;
		assert!(a.x > 0.1, "first I on the CCW half, x={}", a.x);
		assert!(b.x < -0.1, "return I on the other half, x={}", b.x);
		assert!(well.mid_landings().len() >= 1, "turnaround needs an interior landing");
		for s in well.stairs() {
			let p = s.placement.translation;
			assert!(well.well().contains_xz(p.x, p.z), "I should stay in the well, {p:?}");
			let Stair::Straight(g) = &s.geometry else {
				panic!("run-and-landing should emit Straight flights");
			};
			assert!(g.length > 0.4, "two walkable pads must leave a real I, length={}", g.length);
		}
	}

	#[test]
	fn run_and_landing_omits_walk_off_path_when_asked() {
		let well = run_well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
		)
		.with_upper_landing(false);
		assert!(well.upper_landing().is_none());
		assert_eq!(
			well.mid_landings().len(),
			1,
			"routing switchback keeps the interior turnaround; walk-off path drops"
		);
	}
}
