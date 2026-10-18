//! Richmond urbanization Hopscotch + leaf selection.

use bevy::math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::Id;
use procedural_common::{Bounds2, BucketThrow, NoiseConfig, NoiseParams, UnitRange};

use crate::guillotine::{guillotine_partition, UrbanizationGuillotineParams};
use crate::hopscotch::{select, HopscotchNode};
use crate::{
	UrbanDevelopmentKind, UrbanizationExtent, UrbanizationKind, UrbanizationRecipe,
	WeightedDevelopment,
};

/// Default hop-budget range (node weights are typically 0.05–0.70).
pub const DEFAULT_HOP_BUDGET: UnitRange = UnitRange::new(0.0, 3.0);

const LEAF_THROW_LANE: Vec3 = Vec3::new(61.0, 0.0, 0.0);

/// One guillotine leaf with a selected development kind.
#[derive(Debug, Clone, PartialEq)]
pub struct DevelopmentLeaf {
	/// Thin Y (`0..1`) so [`Id::from_cell`] is a stable leaf identity.
	pub bounds: Aabb3d,
	pub kind: UrbanDevelopmentKind,
}

impl DevelopmentLeaf {
	pub fn id(&self) -> Id {
		Id::from_cell(self.bounds)
	}
}

/// Hopscotch kind plus guillotine leaves for one urbanization cell.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedUrbanization {
	pub extent: UrbanizationExtent,
	pub kind: UrbanizationKind,
	/// Empty when [`UrbanizationKind::None`] (short-circuit before guillotine).
	pub leaves: Vec<DevelopmentLeaf>,
}

impl SelectedUrbanization {
	pub fn leaf(&self, id: Id) -> Option<&DevelopmentLeaf> {
		self.leaves.iter().find(|leaf| leaf.id() == id)
	}
}

/// Authored Richmond urbanization Hopscotch graph.
///
/// Anchor weights favour empty land (`None = 0.70`); each other kind is `0.05`.
/// Edges mildly self-loop and soft-bias toward `None` among neighbors.
pub fn urbanization_hopscotch() -> Vec<HopscotchNode<UrbanizationKind>> {
	use UrbanizationKind::*;
	// Soft bias toward None among neighbors; mild self-loops.
	const NONE_W: f32 = 1.5;
	const NEIGH_W: f32 = 1.0;
	const SELF_W: f32 = 0.75;
	vec![
		HopscotchNode::new(
			0.70,
			None,
			vec![(None, NONE_W), (RuralLife, NEIGH_W), (Townships, NEIGH_W), (Frontier, NEIGH_W)],
		),
		HopscotchNode::new(
			0.05,
			MixedAgeCity,
			vec![
				(None, NONE_W),
				(Townships, NEIGH_W),
				(Colony, NEIGH_W),
				(ModernCity, NEIGH_W),
				(MixedAgeCity, SELF_W),
			],
		),
		HopscotchNode::new(
			0.05,
			ModernCity,
			vec![
				(MixedAgeCity, NEIGH_W),
				(None, NONE_W),
				(RuralLife, NEIGH_W),
				(ModernCity, SELF_W),
			],
		),
		HopscotchNode::new(
			0.05,
			RuralLife,
			vec![(None, NONE_W), (Townships, NEIGH_W), (RuralLife, SELF_W)],
		),
		HopscotchNode::new(
			0.05,
			Townships,
			vec![(None, NONE_W), (Frontier, NEIGH_W), (Colony, NEIGH_W), (Townships, SELF_W)],
		),
		HopscotchNode::new(
			0.05,
			Frontier,
			vec![(RuralLife, NEIGH_W), (None, NONE_W), (Colony, NEIGH_W), (Frontier, SELF_W)],
		),
		HopscotchNode::new(
			0.05,
			Colony,
			vec![(RuralLife, NEIGH_W), (None, NONE_W), (MixedAgeCity, NEIGH_W), (Colony, SELF_W)],
		),
	]
}

/// Hopscotch-select an urbanization kind at the cell center.
pub fn select_kind(extent: UrbanizationExtent, noise: NoiseParams) -> UrbanizationKind {
	select(&urbanization_hopscotch(), DEFAULT_HOP_BUDGET, noise, extent.center())
		.unwrap_or(UrbanizationKind::None)
}

/// Hopscotch plus adaptive guillotine + per-leaf Bucket Throw.
pub fn select_cell(extent: UrbanizationExtent, noise: NoiseParams) -> SelectedUrbanization {
	select_cell_as(extent, noise, select_kind(extent, noise))
}

/// Guillotine + Bucket Throw for a forced urbanization kind (playground pin).
pub fn select_cell_as(
	extent: UrbanizationExtent,
	noise: NoiseParams,
	kind: UrbanizationKind,
) -> SelectedUrbanization {
	if kind == UrbanizationKind::None {
		return SelectedUrbanization { extent, kind, leaves: Vec::new() };
	}
	let recipe = kind.recipe();
	let seed = noise.seed as u32;
	let params = UrbanizationGuillotineParams::default().with_seed(seed);
	let min = extent.min();
	let max = extent.max();
	let bounds2 = Bounds2::from_xz(min.x, min.z, max.x, max.z);
	let leaves = guillotine_partition(bounds2, &params)
		.into_iter()
		.map(|leaf| throw_leaf(&recipe, noise, leaf))
		.collect();
	SelectedUrbanization { extent, kind, leaves }
}

fn throw_leaf(recipe: &UrbanizationRecipe, noise: NoiseParams, leaf: Bounds2) -> DevelopmentLeaf {
	let kind = throw_development(&recipe.developments, noise, leaf_center(leaf))
		.unwrap_or(UrbanDevelopmentKind::Empty);
	let bounds = Aabb3d::from_min_max(
		Vec3::new(leaf.min.x, 0.0, leaf.min.y),
		Vec3::new(leaf.max.x, 1.0, leaf.max.y),
	);
	DevelopmentLeaf { bounds, kind }
}

fn leaf_center(leaf: Bounds2) -> Vec3 {
	let c = leaf.center();
	Vec3::new(c.x, 0.0, c.y)
}

fn throw_development(
	buckets: &[WeightedDevelopment],
	noise: NoiseParams,
	position: Vec3,
) -> Option<UrbanDevelopmentKind> {
	if buckets.is_empty() {
		return None;
	}
	let throw = BucketThrow::from_weights(buckets.iter().map(|b| b.weight), 0.0);
	let n = NoiseConfig::new(noise);
	let sample = n.sample_3d(position + LEAF_THROW_LANE + crate::hopscotch::SAMPLE_ORIGIN)
		* throw.total_weight();
	let index = throw.select(sample)?;
	buckets.get(index).map(|b| b.kind)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn richmond_graph_covers_every_kind() -> Result<()> {
		let nodes = urbanization_hopscotch();
		assert_eq!(nodes.len(), UrbanizationKind::ALL.len());
		for kind in UrbanizationKind::ALL {
			assert!(nodes.iter().any(|n| n.item == *kind));
		}
		Ok(())
	}

	#[test]
	fn cell_selection_is_deterministic() -> Result<()> {
		let noise = NoiseParams::from_scalar(3.0, 0.005, 1.0, 1);
		let cell = UrbanizationExtent::default_cell();
		assert_eq!(select_cell(cell, noise), select_cell(cell, noise));
		Ok(())
	}

	#[test]
	fn none_short_circuits_to_empty_leaves() -> Result<()> {
		let selected = SelectedUrbanization {
			extent: UrbanizationExtent::default_cell(),
			kind: UrbanizationKind::None,
			leaves: Vec::new(),
		};
		assert!(selected.leaves.is_empty());
		assert_eq!(selected.kind.recipe().developments.len(), 0);
		Ok(())
	}

	#[test]
	fn populated_kind_produces_leaves() -> Result<()> {
		let noise = NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1);
		let mut found = None;
		for ix in -2..=2 {
			for iz in -2..=2 {
				let cell = UrbanizationExtent::from_cell_index(ix, iz);
				let selected = select_cell(cell, noise);
				if selected.kind != UrbanizationKind::None {
					found = Some(selected);
					break;
				}
			}
			if found.is_some() {
				break;
			}
		}
		let selected = found.ok_or_else(|| anyhow::anyhow!("expected a non-None cell nearby"))?;
		assert!(!selected.leaves.is_empty());
		for leaf in &selected.leaves {
			let _ = leaf.id();
		}
		Ok(())
	}

	#[test]
	fn select_cell_as_pins_kind() -> Result<()> {
		let noise = NoiseParams::from_scalar(1.0, 0.0005, 1.0, 1);
		let cell = UrbanizationExtent::default_cell();
		let selected = select_cell_as(cell, noise, UrbanizationKind::Frontier);
		assert_eq!(selected.kind, UrbanizationKind::Frontier);
		assert!(!selected.leaves.is_empty());
		Ok(())
	}
}
