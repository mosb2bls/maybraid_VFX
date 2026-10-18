//! Placement-contract regressions for [#452](https://github.com/ramate-io/maybraid/issues/452).
//!
//! VegetationComponents palms keep crown anchors in tree-local space; root transform
//! composition is handled by the LodScene host (not ChildOf mesh roots as in RenderItem).

use anyhow::Result;
use lod::scene::LodSceneLevel;
use sbs_geometry::DatePalmSbs;
use sbs_trees::{DatePalmParams, PalmBushParams};
use vegetation_components::VegetationComponents;

#[test]
fn palm_bush_ring_anchors_are_tree_local() -> Result<()> {
	let bush = PalmBushParams::default().build();
	let ring_count = bush.geometry.crown.ring_count;
	for ring in 0..ring_count {
		let anchor = bush.geometry.crown_ring_position(ring);
		assert!(
			anchor.y >= bush.geometry.crown_origin().y - 1e-4,
			"ring {ring} should sit at/above crown origin"
		);
		assert!(
			anchor.length() < bush.geometry.height() * 2.0,
			"ring {ring} anchor {anchor} looks world-absolute"
		);
	}
	Ok(())
}

#[test]
fn palm_bush_high_emits_frond_collections() -> Result<()> {
	let bush = PalmBushParams::default().build();
	let nodes = bush.foliage_nodes_for_level(LodSceneLevel::High).flatten();
	assert!(!nodes.is_empty());
	assert!(nodes[0].geometry.as_frond_collection().is_some());
	Ok(())
}

#[test]
fn date_palm_crown_rings_stack_from_trunk_tip() -> Result<()> {
	let palm = DatePalmParams::default().build();
	let tip = DatePalmSbs::trunk_tip_from_chain(&palm.chain);
	let first = palm.geometry.crown_ring_position(&palm.chain, 0);
	assert!(
		first.distance(tip) < palm.geometry.height() * 0.25,
		"first crown ring should sit near trunk tip {tip}, got {first}"
	);
	Ok(())
}

#[test]
fn date_palm_emits_trunk_sticks() -> Result<()> {
	let palm = DatePalmParams::default().build();
	let sticks = palm.stick_nodes_for_level(LodSceneLevel::High).flatten();
	assert!(!sticks.is_empty(), "date palm should emit trunk sticks");
	Ok(())
}
