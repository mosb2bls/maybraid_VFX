//! Initial Chico vegetation Hopscotch ([RFC-183 §3.5.5]).

use bevy_math::Vec3;
use procedural_common::{NoiseParams, UnitRange};

use crate::hopscotch::{select, HopscotchNode};
use crate::layer::select_layers;
use crate::{ForestExtent, LayeringKind, SelectedLayers};

/// Default hop-budget range (node weights are typically 0.75–4.0).
pub const DEFAULT_HOP_BUDGET: UnitRange = UnitRange::new(0.0, 3.0);

/// Authored Chico vegetation Hopscotch graph.
pub fn chico_hopscotch() -> Vec<HopscotchNode<LayeringKind>> {
	use LayeringKind::*;
	vec![
		HopscotchNode::new(
			2.0,
			LushJungle,
			vec![
				(TrapThicket, 1.5),
				(LiamsSummer, 1.0),
				(Kumulipo, 1.0),
				(OpenTropics, 0.5),
				(Riparian, 0.5),
				(DamasEdge, 0.1),
				(SunsBarren, 0.1),
				(LushJungle, 0.5),
			],
		),
		HopscotchNode::new(
			4.0,
			Riparian,
			vec![
				(Riparian, 0.5),
				(Storybook, 1.0),
				(Meadowland, 1.0),
				(MiRobles, 0.75),
				(FruitPlains, 0.75),
				(LushJungle, 0.5),
			],
		),
		HopscotchNode::new(
			1.0,
			Taiga,
			vec![
				(Seceda, 1.25),
				(OldNevada, 0.75),
				(TemperateHoly, 0.5),
				(OldSteppe, 0.5),
				(Taiga, 1.0),
			],
		),
		HopscotchNode::new(
			1.0,
			LiamsSummer,
			vec![
				(WestMaui, 1.25),
				(OpenTropics, 1.0),
				(DamasEdge, 0.75),
				(Bush, 0.75),
				(Kumulipo, 0.5),
			],
		),
		HopscotchNode::new(
			2.0,
			OwlsDesert,
			vec![
				(SunsBarren, 1.0),
				(DamasEdge, 1.0),
				(OldNevada, 0.75),
				(Bush, 0.5),
				(SteppeDown, 0.5),
				(OwlsDesert, 2.0),
			],
		),
		HopscotchNode::new(
			2.0,
			MiRobles,
			vec![
				(UpperPark, 1.25),
				(Meadowland, 1.0),
				(Riparian, 0.5),
				(FruitPlains, 0.5),
				(SteppeDown, 0.5),
				(Bush, 0.5),
				(MiRobles, 1.0),
			],
		),
		HopscotchNode::new(
			1.0,
			Seceda,
			vec![
				(Taiga, 1.25),
				(OldNevada, 1.0),
				(SteppeDown, 0.75),
				(TemperateHoly, 0.5),
				(SunsBarren, 0.5),
			],
		),
		HopscotchNode::new(
			0.75,
			Kumulipo,
			vec![
				(LushJungle, 1.0),
				(OpenTropics, 1.0),
				(WestMaui, 0.75),
				(LiamsSummer, 0.5),
				(DamasEdge, 0.5),
			],
		),
		HopscotchNode::new(
			1.0,
			Waiguo,
			vec![
				(AgTown, 1.5),
				(FruitPlains, 1.25),
				(Storybook, 0.5),
				(Riparian, 0.5),
				(MiRobles, 0.5),
				(Waiguo, 1.0),
			],
		),
		HopscotchNode::new(
			0.75,
			AgTown,
			vec![
				(Waiguo, 0.25),
				(FruitPlains, 1.0),
				(Meadowland, 0.75),
				(SunsBarren, 0.25),
				(AgTown, 2.0),
			],
		),
		HopscotchNode::new(
			2.0,
			SunsBarren,
			vec![
				(OwlsDesert, 1.0),
				(SteppeDown, 1.0),
				(OldSteppe, 0.75),
				(OldNevada, 0.5),
				(Meadowland, 0.25),
			],
		),
		HopscotchNode::new(
			0.75,
			TemperateHoly,
			vec![
				(Riparian, 2.0),
				(Taiga, 0.75),
				(Seceda, 0.5),
				(Storybook, 0.5),
				(Meadowland, 0.5),
				(MiRobles, 0.5),
			],
		),
		HopscotchNode::new(
			2.0,
			OldSteppe,
			vec![
				(SteppeDown, 1.25),
				(Meadowland, 1.0),
				(SunsBarren, 0.75),
				(UpperPark, 0.75),
				(OldNevada, 0.5),
				(Kumulipo, 0.25),
				(OldSteppe, 2.0),
			],
		),
		HopscotchNode::new(
			0.75,
			TrapThicket,
			vec![
				(LushJungle, 1.5),
				(OpenTropics, 0.75),
				(Kumulipo, 0.5),
				(Storybook, 0.25),
				(TrapThicket, 1.0),
			],
		),
		HopscotchNode::new(
			2.0,
			Bush,
			vec![
				(UpperPark, 1.0),
				(SteppeDown, 1.0),
				(WestMaui, 0.75),
				(DamasEdge, 0.75),
				(MiRobles, 0.5),
				(OwlsDesert, 0.5),
				(SunsBarren, 0.5),
				(Bush, 3.0),
			],
		),
		HopscotchNode::new(
			1.0,
			OldNevada,
			vec![
				(OwlsDesert, 1.0),
				(Seceda, 1.0),
				(Taiga, 0.75),
				(SunsBarren, 0.75),
				(SteppeDown, 0.75),
			],
		),
		HopscotchNode::new(
			2.0,
			Storybook,
			vec![
				(Riparian, 1.0),
				(Meadowland, 0.75),
				(TemperateHoly, 0.5),
				(Waiguo, 0.5),
				(LushJungle, 0.25),
				(Storybook, 1.0),
			],
		),
		HopscotchNode::new(
			1.0,
			Meadowland,
			vec![
				(Meadowland, 0.5),
				(OldSteppe, 1.0),
				(MiRobles, 1.0),
				(Riparian, 0.75),
				(FruitPlains, 0.75),
				(UpperPark, 0.75),
				(Storybook, 0.5),
			],
		),
		HopscotchNode::new(
			1.0,
			FruitPlains,
			vec![
				(Waiguo, 0.5),
				(AgTown, 1.0),
				(Meadowland, 1.0),
				(MiRobles, 0.75),
				(Riparian, 0.5),
			],
		),
		HopscotchNode::new(
			0.75,
			DamasEdge,
			vec![
				(OwlsDesert, 1.0),
				(Bush, 0.75),
				(LiamsSummer, 0.75),
				(OpenTropics, 0.5),
				(WestMaui, 0.5),
			],
		),
		HopscotchNode::new(
			1.25,
			OpenTropics,
			vec![
				(WestMaui, 1.0),
				(LiamsSummer, 1.0),
				(Kumulipo, 0.75),
				(LushJungle, 1.5),
				(DamasEdge, 0.5),
				(OpenTropics, 1.0),
			],
		),
		HopscotchNode::new(
			1.25,
			WestMaui,
			vec![
				(OpenTropics, 1.0),
				(LiamsSummer, 1.0),
				(Bush, 0.75),
				(DamasEdge, 0.5),
				(SteppeDown, 0.5),
			],
		),
		HopscotchNode::new(
			1.25,
			UpperPark,
			vec![
				(MiRobles, 1.0),
				(Bush, 1.0),
				(SteppeDown, 1.0),
				(OldSteppe, 0.75),
				(Meadowland, 0.75),
			],
		),
		HopscotchNode::new(
			1.25,
			SteppeDown,
			vec![
				(UpperPark, 1.0),
				(OldSteppe, 1.0),
				(Bush, 1.0),
				(SunsBarren, 0.75),
				(OldNevada, 0.5),
				(WestMaui, 0.5),
				(SteppeDown, 2.0),
			],
		),
	]
}

/// Hopscotch-select a Chico layering at `position`.
pub fn select_layering(noise: NoiseParams, position: Vec3) -> LayeringKind {
	select(&chico_hopscotch(), DEFAULT_HOP_BUDGET, noise, position)
		.unwrap_or(LayeringKind::Riparian)
}

/// Hopscotch plus per-layer Bucket Throw for one forest cell.
///
/// Does not apply [`vegetation_groves::ForestGroveBiases`] — those stay default until the
/// full RFC bias set is implemented.
pub fn select_cell(extent: ForestExtent, noise: NoiseParams) -> SelectedLayers {
	let layering = select_layering(noise, extent.center()).layering();
	select_layers(&layering, noise, extent.center())
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn chico_graph_covers_every_layering() -> Result<()> {
		let nodes = chico_hopscotch();
		assert_eq!(nodes.len(), 24);
		Ok(())
	}

	#[test]
	fn cell_selection_is_deterministic() -> Result<()> {
		let noise = NoiseParams::from_scalar(3.0, 0.005, 1.0, 1);
		let cell = ForestExtent::default_cell();
		assert_eq!(select_cell(cell, noise), select_cell(cell, noise));
		Ok(())
	}

	#[test]
	fn origin_cell_default_forest_noise_selects_a_grove() -> Result<()> {
		let noise = NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1);
		let layers = select_cell(ForestExtent::default_cell(), noise);
		assert!(
			layers.tufts.is_some()
				|| layers.understory.is_some()
				|| layers.lower_canopy.is_some()
				|| layers.upper_canopy.is_some(),
			"origin cell was empty: {layers:?}"
		);
		Ok(())
	}
}
