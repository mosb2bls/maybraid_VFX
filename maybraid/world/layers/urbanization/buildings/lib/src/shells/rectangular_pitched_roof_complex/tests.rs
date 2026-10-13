use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

use crate::openings::{MapsOpenings, OpeningId, OpeningLabel};
use crate::ClippedStripPiece;

use super::geometry::{LongAxis, VolumeCandidate};
use super::topology::resolve_junctions;
use super::{EndCap, Overhang, RectangularPitchedRoofComplexParams, RidgeJunction};

#[test]
fn single_box_long_axis_and_no_valleys() {
	let complex = RectangularPitchedRoofComplexParams::single(10.0, 6.0, 2.0, 4.0)
		.end_cap(EndCap::Hip)
		.build();
	assert_eq!(complex.roofs().len(), 1);
	assert!(complex.valleys().is_empty());
	let ridge = complex.roofs()[0].params().halves[0].ridge_line;
	assert!((ridge.0.z - ridge.1.z).abs() < 1e-4, "ridge along X");
	assert!(ridge.0.x < ridge.1.x);
}

#[test]
fn single_gable_extends_free_ends() {
	let complex = RectangularPitchedRoofComplexParams::single(10.0, 6.0, 2.0, 4.0)
		.end_cap(EndCap::Gable { ridge: Overhang::Fixed(0.5), eave: Overhang::Fixed(0.4) })
		.build();
	assert_eq!(complex.roofs().len(), 1);
	let half = &complex.roofs()[0].params().halves[0];
	assert_eq!(half.draw_in_half_gable_end, (true, true));
	assert_eq!(half.draw_in_half_hip, (false, false));
	// Wall stays at the massing end; ridge/eave project past it (barge overhang).
	let wall0 = half.wall_line.0.x;
	let wall1 = half.wall_line.1.x;
	assert!((wall0 - (-5.0)).abs() < 1e-3, "wall min {wall0}");
	assert!((wall1 - 5.0).abs() < 1e-3, "wall max {wall1}");
	assert!((half.eave_line.0.x - (-5.4)).abs() < 1e-3);
	assert!((half.eave_line.1.x - 5.4).abs() < 1e-3);
	assert!((half.ridge_line.0.x - (-5.5)).abs() < 1e-3);
	assert!((half.ridge_line.1.x - 5.5).abs() < 1e-3);
}

#[test]
fn stepped_presets_vary_ridge_and_eave_heights() {
	let ridge_step = RectangularPitchedRoofComplexParams::l_shape_stepped_ridge().build();
	// Junction snaps to the lower ridge; the taller free end keeps its box top.
	let mut saw_low_junction = false;
	let mut saw_tall_free = false;
	for roof in ridge_step.roofs() {
		let (a, b) = roof.params().halves[0].ridge_line;
		for p in [a, b] {
			if (p.y - 4.2).abs() < 1e-3 {
				saw_low_junction = true;
			}
			if (p.y - 5.5).abs() < 1e-3 {
				saw_tall_free = true;
			}
		}
	}
	assert!(saw_low_junction, "expected junction at lower ridge 4.2");
	assert!(saw_tall_free, "expected taller free end at 5.5");
	let v = &ridge_step.valleys()[0];
	assert!(
		(v.ridge_point.y - 4.2).abs() < 1e-3,
		"valley should meet at lowest ridge, got {}",
		v.ridge_point.y
	);

	let eave_step = RectangularPitchedRoofComplexParams::l_shape_stepped_eave().build();
	let eave_ys: Vec<f32> = eave_step
		.roofs()
		.iter()
		.flat_map(|r| {
			let (a, b) = r.params().halves[0].eave_line;
			[a.y, b.y]
		})
		.collect();
	assert!(eave_ys.iter().any(|y| (y - 2.0).abs() < 1e-3), "missing bar eave 2.0 in {eave_ys:?}");
	assert!(eave_ys.iter().any(|y| (y - 3.2).abs() < 1e-3), "missing stem eave 3.2 in {eave_ys:?}");
	assert!(!eave_step.valleys().is_empty());

	let t = RectangularPitchedRoofComplexParams::t_shape_stepped().build();
	assert_eq!(t.roofs().len(), 2);
	assert!(!t.valleys().is_empty());
}

#[test]
fn run_up_blends_junction_ridge_height() {
	let low = RectangularPitchedRoofComplexParams::l_shape_stepped_ridge()
		.ridge_junction(RidgeJunction::RunUp(0.0))
		.build();
	assert!((low.valleys()[0].ridge_point.y - 4.2).abs() < 1e-3);

	let mid = RectangularPitchedRoofComplexParams::l_shape_stepped_ridge()
		.ridge_junction(RidgeJunction::RunUp(0.5))
		.build();
	assert!((mid.valleys()[0].ridge_point.y - 4.85).abs() < 1e-3);

	let high = RectangularPitchedRoofComplexParams::l_shape_stepped_ridge()
		.ridge_junction(RidgeJunction::RunUp(1.0))
		.build();
	assert!((high.valleys()[0].ridge_point.y - 5.5).abs() < 1e-3);
}

#[test]
fn hall_and_bays_has_three_t_junctions() {
	let complex = RectangularPitchedRoofComplexParams::hall_and_bays().build();
	assert_eq!(complex.roofs().len(), 4);
	// Each bay forms a T → two concave corners; three bays → six valleys.
	assert!(complex.valleys().len() >= 6, "expected ≥6 valleys, got {}", complex.valleys().len());
}

#[test]
fn t_bar_keeps_full_eaves_stems_strip_to_valley() {
	let complex = RectangularPitchedRoofComplexParams::hall_and_bays().build();
	// Hall (T-bar): full ~28m eave span — uncovered extents stay drawn.
	let hall = &complex.roofs()[0];
	for half in &hall.params().halves {
		let span_x = (half.eave_line.1.x - half.eave_line.0.x).abs();
		assert!(span_x > 27.5, "hall eave should stay full, got {span_x}");
	}
	// Bays (stems): facing eaves strip back toward the hall valley.
	for bay in &complex.roofs()[1..] {
		let mut min_span = f32::MAX;
		for half in &bay.params().halves {
			let span = (half.eave_line.1 - half.eave_line.0).length();
			min_span = min_span.min(span);
		}
		assert!(
			min_span < 9.0,
			"bay facing eave should strip back under hall, min span {min_span}"
		);
	}
}

#[test]
fn l_shape_marks_junction_and_builds_valley() {
	let complex = RectangularPitchedRoofComplexParams::l_shape().build();
	assert_eq!(complex.roofs().len(), 2);
	assert!(!complex.valleys().is_empty(), "expected at least one valley at the L corner");
	let v = complex.valleys()[0];
	assert!(v.ridge_point.y > v.eave_point.y);
	// Inner corner of default L is near (+2, +2) in XZ before overhang.
	assert!(v.eave_point.x > 1.5 && v.eave_point.z > 1.5);

	// Outside hip corner: outer eaves meet near (−2−oh, −2−oh).
	let oh = 0.3;
	let mut found_outer = false;
	for roof in complex.roofs() {
		for half in &roof.params().halves {
			for end in [half.eave_line.0, half.eave_line.1] {
				if (end.x - (-2.0 - oh)).abs() < 1e-2 && (end.z - (-2.0 - oh)).abs() < 1e-2 {
					found_outer = true;
				}
			}
		}
	}
	assert!(found_outer, "expected outer eaves to meet at the convex L corner");

	// Ridges stay level at box top.
	for roof in complex.roofs() {
		let (a, b) = roof.params().halves[0].ridge_line;
		assert!((a.y - b.y).abs() < 1e-3);
		assert!((a.y - 4.5).abs() < 1e-3);
	}
}

#[test]
fn t_shape_builds_valleys() {
	let complex = RectangularPitchedRoofComplexParams::t_shape().build();
	assert_eq!(complex.roofs().len(), 2);
	assert!(
		complex.valleys().len() >= 2,
		"T should yield two concave corners, got {}",
		complex.valleys().len()
	);
}

#[test]
fn disjoint_boxes_have_no_valleys() {
	let complex = RectangularPitchedRoofComplexParams::new(vec![
		Aabb3d::from_min_max(Vec3::new(0.0, 2.0, 0.0), Vec3::new(6.0, 4.0, 3.0)),
		Aabb3d::from_min_max(Vec3::new(20.0, 2.0, 0.0), Vec3::new(23.0, 4.0, 8.0)),
	])
	.build();
	assert_eq!(complex.roofs().len(), 2);
	assert!(complex.valleys().is_empty());
}

#[test]
fn demo_presets_valley_expectations() {
	let disjoint = RectangularPitchedRoofComplexParams::disjoint().build();
	assert_eq!(disjoint.roofs().len(), 4);
	assert!(disjoint.valleys().is_empty());

	let mixed = RectangularPitchedRoofComplexParams::mixed().build();
	assert_eq!(mixed.roofs().len(), 5);
	assert!(!mixed.valleys().is_empty(), "mixed cluster should still form valleys");

	let ring = RectangularPitchedRoofComplexParams::ring().build();
	assert_eq!(ring.roofs().len(), 4);
	assert!(
		ring.valleys().len() >= 4,
		"ring should have an inner L valley per corner, got {}",
		ring.valleys().len()
	);

	let ring_stepped = RectangularPitchedRoofComplexParams::ring_stepped().build();
	assert_eq!(ring_stepped.roofs().len(), 4);
	assert!(ring_stepped.valleys().len() >= 4);

	let p = RectangularPitchedRoofComplexParams::p_shape().build();
	assert_eq!(p.roofs().len(), 4);
	assert!(p.valleys().len() >= 4);

	let coaxial = RectangularPitchedRoofComplexParams::coaxial_parallel().build();
	assert_eq!(
		coaxial.roofs().len(),
		3,
		"coaxial nested should keep the higher cap and two lower wings"
	);
	assert_eq!(
		coaxial.valleys().len(),
		2,
		"each wing should meet a cap end-gable, got {}",
		coaxial.valleys().len()
	);

	let cross = RectangularPitchedRoofComplexParams::pathological_cross().build();
	assert_eq!(cross.roofs().len(), 4, "full + cross should decompose into four arm roofs");
	assert_eq!(cross.valleys().len(), 4, "four L valleys at the ridge crossing");
}

#[test]
fn junction_detection_l() {
	let mut vols = vec![
		VolumeCandidate::from_aabb(
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(8.0, 4.5, 2.0)),
			Overhang::Fixed(0.3),
		),
		VolumeCandidate::from_aabb(
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(2.0, 4.5, 8.0)),
			Overhang::Fixed(0.3),
		),
	];
	assert_eq!(vols[0].long_axis, LongAxis::X);
	assert_eq!(vols[1].long_axis, LongAxis::Z);
	let junctions = resolve_junctions(&mut vols);
	assert_eq!(junctions.perp.len(), 1);
	assert!(junctions.coaxial.is_empty());
	assert!(!vols[0].end_free[0] || !vols[1].end_free[0]);
}

#[test]
fn full_cross_decomposes_before_junctions() {
	use super::decompose::decompose_volumes;
	let arms = decompose_volumes(&[
		Aabb3d::from_min_max(Vec3::new(-10.0, 2.5, -2.0), Vec3::new(10.0, 4.5, 2.0)),
		Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -10.0), Vec3::new(2.0, 4.5, 10.0)),
	]);
	assert_eq!(arms.len(), 4);
	let mut vols: Vec<_> = arms
		.into_iter()
		.map(|a| VolumeCandidate::from_aabb(a, Overhang::Fixed(0.3)))
		.collect();
	let junctions = resolve_junctions(&mut vols);
	assert_eq!(junctions.perp.len(), 4);
	assert!(junctions.perp.iter().all(|c| c.end_a.is_some() && c.end_b.is_some()));
}

#[test]
fn pitch_opening_assigns_to_nearest_roof_and_clips() {
	let complex = RectangularPitchedRoofComplexParams::l_shape()
		.with_pitch_opening(0, 0, 0.55, 0.45, 1.4, 1.0, "sky", OpeningLabel::Aperture)
		.build();
	assert!(complex.mapped_opening(&OpeningId::new("sky")).is_some());
	assert_eq!(complex.openings().len(), 1);
	let clipped = complex.roofs().iter().any(|r| {
		r.pitches()
			.iter()
			.any(|p| matches!(p.pieces()[0], ClippedStripPiece::Clipped(_)))
	});
	assert!(clipped, "expected a clipped pitch on the nearest roof");
}

#[test]
fn coaxial_hip_meets_wing_ridge_on_hip_edge() {
	let complex = RectangularPitchedRoofComplexParams::coaxial_parallel()
		.end_cap(EndCap::Hip)
		.build();
	assert_eq!(complex.roofs().len(), 3);

	// Cap hip inset = 3.5 + 0.3 → apex at ±2.2. Hip edge runs to end-wall drop
	// (±6, 2.8, 0). Lower ridge y=4 meets that edge at t=(4-5.5)/(2.8-5.5).
	let t = (4.0 - 5.5) / (2.8 - 5.5);
	let meet_neg = -2.2 + t * (-6.0 - (-2.2));
	let meet_pos = 2.2 + t * (6.0 - 2.2);

	let mut saw_neg_wing = false;
	let mut saw_pos_wing = false;
	for roof in complex.roofs() {
		let half = &roof.params().halves[0];
		let (r0, r1) = half.ridge_line;
		let (e0, e1) = half.eave_line;
		if (r0.y - 4.0).abs() > 0.1 {
			continue;
		}
		if r0.x < -6.0 {
			assert!((e1.x - (-6.0)).abs() < 1e-3, "eave at wall, got {}", e1.x);
			assert!(
				(r1.x - meet_neg).abs() < 1e-3,
				"ridge on hip edge, got {} want {meet_neg}",
				r1.x
			);
			assert!(
				r1.x > -6.0 + 1e-3 && r1.x < -2.2 - 1e-3,
				"meet should lie between wall and apex, got {}",
				r1.x
			);
			saw_neg_wing = true;
		} else if r1.x > 6.0 {
			assert!((e0.x - 6.0).abs() < 1e-3, "eave at wall, got {}", e0.x);
			assert!(
				(r0.x - meet_pos).abs() < 1e-3,
				"ridge on hip edge, got {} want {meet_pos}",
				r0.x
			);
			saw_pos_wing = true;
		}
	}
	assert!(saw_neg_wing && saw_pos_wing, "expected both lower wings");
}

#[test]
fn coaxial_nested_decomposes_to_end_meets() {
	use super::decompose::decompose_volumes;
	let parts = decompose_volumes(&[
		Aabb3d::from_min_max(Vec3::new(-10.0, 2.0, -2.0), Vec3::new(10.0, 4.0, 2.0)),
		Aabb3d::from_min_max(Vec3::new(-6.0, 2.8, -3.5), Vec3::new(6.0, 5.5, 3.5)),
	]);
	assert_eq!(parts.len(), 3);
	let mut vols: Vec<_> = parts
		.into_iter()
		.map(|a| VolumeCandidate::from_aabb(a, Overhang::Fixed(0.3)))
		.collect();
	let junctions = resolve_junctions(&mut vols);
	assert!(junctions.perp.is_empty());
	assert_eq!(junctions.coaxial.len(), 2);
	for m in &junctions.coaxial {
		assert!(!vols[m.vol_run].end_free[m.run_end]);
		assert!(vols[m.vol_cap].end_free[0] && vols[m.vol_cap].end_free[1]);
	}
}

#[test]
fn plan_holes_cut_a_courtyard_ring_corner() {
	let hole = Aabb3d::from_min_max(Vec3::new(4.0, 2.0, 4.0), Vec3::new(8.0, 5.0, 8.0));
	let complex = RectangularPitchedRoofComplexParams::ring().with_plan_holes([hole]).build();
	let volumes = &complex.params().volumes;
	let keep = Vec3::new(6.0, 0.0, 6.0);
	assert!(
		!volumes
			.iter()
			.any(|v| keep.x > v.min.x && keep.x < v.max.x && keep.z > v.min.z && keep.z < v.max.z),
		"NE corner should be a hole"
	);
	let gallery = Vec3::new(0.0, 0.0, 6.0);
	assert!(
		volumes.iter().any(|v| gallery.x > v.min.x
			&& gallery.x < v.max.x
			&& gallery.z > v.min.z
			&& gallery.z < v.max.z),
		"north gallery should remain"
	);
	assert!(!complex.roofs().is_empty());
}
