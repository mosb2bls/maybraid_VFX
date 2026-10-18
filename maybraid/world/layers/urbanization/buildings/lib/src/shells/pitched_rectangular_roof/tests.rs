use bevy_math::{Vec2, Vec3};
use building_components::BuildingComponents;
use lod::scene::LodSceneLevel;

use crate::openings::{MapsOpenings, OpeningId, OpeningLabel, Openings};
use crate::paneling::clipped_ruled_strip::ClippedStripPiece;

use super::openings::centered_pitch_clip;
use super::{PitchedRoof, PitchedRoofParams, RoofHalf};

fn assert_vec3_close(got: Vec3, want: Vec3) {
	assert!((got - want).length() < 1e-4, "got {got:?} want {want:?}");
}

#[test]
fn rectangular_hip_shared_ridge_and_four_hips() {
	let params = PitchedRoofParams::rectangular_hip(Vec2::new(10.0, 6.0), 4.0, 2.5, 1.5);
	let roof = PitchedRoof::new(params);

	assert_eq!(roof.params().halves[0].ridge_line, roof.params().halves[1].ridge_line);
	assert_eq!(roof.hip_count(), 4);
	assert_eq!(roof.gable_count(), 0);
	assert!(roof.wall_complexes()[0].is_some());
	assert!(roof.wall_complexes()[1].is_some());
	assert!(roof.openings().is_empty());

	for pitch in roof.pitches() {
		assert!(!pitch.pieces().is_empty());
		assert!(matches!(pitch.pieces()[0], ClippedStripPiece::Solid(_)));
	}

	let eave_pos_z = roof.params().halves[0].eave_line.0.z;
	let eave_neg_z = roof.params().halves[1].eave_line.0.z;
	let mid_z = 0.5 * (eave_pos_z + eave_neg_z);
	assert!((mid_z).abs() < 1e-5);

	for hip in roof.hip_panels() {
		let e = hip.a;
		let r = hip.b;
		let p = hip.c;
		assert!((p.y - 2.5).abs() < 1e-4);
		assert!((p.z - mid_z).abs() < 1e-4);
		assert_vec3_close(p, Vec3::new(e.x, e.y, r.z));
		assert!((p.x - r.x).abs() > 0.5, "hip base should reach past the ridge inset");
	}

	let panels = roof.panel_nodes_for_level(LodSceneLevel::High).flatten().len();
	// 2 pitches × 2 tris + 2 walls × 2 tris + 4 hips = 12
	assert_eq!(panels, 12);
}

#[test]
fn gable_only_emits_end_walling() {
	let footprint = Vec2::new(8.0, 5.0);
	let half_x = footprint.x * 0.5;
	let half_z = footprint.y * 0.5;
	let ridge = (Vec3::new(-half_x, 4.0, 0.0), Vec3::new(half_x, 4.0, 0.0));
	let eave = (Vec3::new(-half_x, 2.0, half_z), Vec3::new(half_x, 2.0, half_z));
	let wall = (Vec3::new(-half_x, 2.0, half_z - 0.2), Vec3::new(half_x, 2.0, half_z - 0.2));
	let pos = RoofHalf::new(ridge, eave, wall)
		.draw_in_wall_line(true)
		.draw_in_half_gable_end((true, true));
	let neg_eave = (Vec3::new(-half_x, 2.0, -half_z), Vec3::new(half_x, 2.0, -half_z));
	let neg_wall =
		(Vec3::new(-half_x, 2.0, -(half_z - 0.2)), Vec3::new(half_x, 2.0, -(half_z - 0.2)));
	let neg = RoofHalf::new(ridge, neg_eave, neg_wall);
	let roof = PitchedRoofParams::new([pos, neg]).build();

	assert_eq!(roof.hip_count(), 0);
	assert_eq!(roof.gable_count(), 4);
	assert!(roof.wall_complexes()[0].is_some());
	assert!(roof.wall_complexes()[1].is_none());
}

#[test]
fn gable_and_hip_coexist_on_same_end() {
	let ridge = (Vec3::new(-2.0, 4.0, 0.0), Vec3::new(2.0, 4.0, 0.0));
	let eave = (Vec3::new(-4.0, 2.0, 3.0), Vec3::new(4.0, 2.0, 3.0));
	let wall = (Vec3::new(-4.0, 2.0, 2.7), Vec3::new(4.0, 2.0, 2.7));
	let half = RoofHalf::new(ridge, eave, wall)
		.draw_in_half_hip((true, false))
		.draw_in_half_gable_end((true, false));
	let other = RoofHalf::new(
		ridge,
		(Vec3::new(-4.0, 2.0, -3.0), Vec3::new(4.0, 2.0, -3.0)),
		(Vec3::new(-4.0, 2.0, -2.7), Vec3::new(4.0, 2.0, -2.7)),
	);
	let roof = PitchedRoofParams::new([half, other]).build();
	assert_eq!(roof.hip_count(), 1);
	assert_eq!(roof.gable_count(), 2);
}

#[test]
fn aperture_clips_nearest_pitch_and_maps() -> anyhow::Result<()> {
	let params = PitchedRoofParams::rectangular_hip(Vec2::new(10.0, 6.0), 4.0, 2.5, 1.5);
	let opening =
		PitchedRoof::pitch_opening(&params.halves[0], 0.5, 0.45, 1.5, 1.0, OpeningLabel::Aperture);
	let roof = params.openings(Openings::new().with("sky", opening)).build();

	assert!(roof.mapped_opening(&OpeningId::new("sky")).is_some());
	assert!(matches!(roof.pitches()[0].pieces()[0], ClippedStripPiece::Clipped(_)));
	assert!(matches!(roof.pitches()[1].pieces()[0], ClippedStripPiece::Solid(_)));
	Ok(())
}

#[test]
fn largest_aperture_wins_per_half() -> anyhow::Result<()> {
	let params = PitchedRoofParams::rectangular_hip(Vec2::new(10.0, 6.0), 4.0, 2.5, 1.5);
	let small =
		PitchedRoof::pitch_opening(&params.halves[0], 0.4, 0.4, 0.6, 0.5, OpeningLabel::Aperture);
	let large =
		PitchedRoof::pitch_opening(&params.halves[0], 0.6, 0.5, 2.0, 1.2, OpeningLabel::Aperture);
	let roof = params
		.openings(Openings::new().with("small", small).with("large", large))
		.build();
	assert!(roof.mapped_opening(&OpeningId::new("large")).is_some());
	assert!(roof.mapped_opening(&OpeningId::new("small")).is_none());
	Ok(())
}

#[test]
fn pitch_clip_is_centered_quad_on_face() {
	let half =
		PitchedRoofParams::rectangular_hip(Vec2::new(10.0, 6.0), 4.0, 2.5, 1.5).halves[0].clone();
	let clip = centered_pitch_clip(&half, 0.5, 0.5, 2.0, 1.0);
	assert_eq!(clip.len(), 4);
	let mid = (clip[0] + clip[1] + clip[2] + clip[3]) * 0.25;
	let face_mid = half.pitch_point(0.5, 0.5);
	assert!((mid - face_mid).length() < 0.2);
}

#[test]
fn skylight_reduces_or_changes_pitch_panels() {
	use building_components::BuildingComponents;
	use lod::scene::LodSceneLevel;
	let base = PitchedRoofParams::rectangular_hip(Vec2::new(10.0, 6.0), 4.0, 2.5, 1.5);
	let solid = PitchedRoof::new(base.clone());
	let opening =
		PitchedRoof::pitch_opening(&base.halves[0], 0.5, 0.45, 1.5, 1.0, OpeningLabel::Aperture);
	let clipped = base.openings(Openings::new().with("sky", opening)).build();
	let solid_n = solid.pitches()[0].panel_nodes_for_level(LodSceneLevel::High).flatten().len();
	let clip_n = clipped.pitches()[0].panel_nodes_for_level(LodSceneLevel::High).flatten().len();
	assert!(
		clip_n > solid_n,
		"expected clip to subdivide pitch (solid={solid_n} clipped={clip_n})"
	);
}

#[test]
fn gable_end_window_maps_and_clips_end_wall() -> anyhow::Result<()> {
	let params = PitchedRoofParams::rectangular_gable(Vec2::new(16.0, 10.0), 7.0, 2.5);
	assert_eq!(params.halves[0].draw_in_half_gable_end, (true, true));
	assert_eq!(params.halves[0].draw_in_half_hip, (false, false));
	let opening =
		PitchedRoof::gable_end_opening(&params.halves, 1, 2.4, 2.0, OpeningLabel::Aperture);
	let roof = params.openings(Openings::new().with("gable_win", opening)).build();
	assert!(roof.mapped_opening(&OpeningId::new("gable_win")).is_some());
	// Pitches stay solid; gable end walling is clipped.
	assert!(matches!(roof.pitches()[0].pieces()[0], ClippedStripPiece::Solid(_)));
	assert!(roof.gable_panels().any(|g| !g.clip.is_empty()));
	Ok(())
}
