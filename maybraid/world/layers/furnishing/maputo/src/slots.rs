//! World-space furniture a generate pass reads from the development index.

use building_components::FurnitureNode;
use furniture_usage_areas::expand_usages;
use lod::hcsg;
use richmond::{Built, BuiltDevelopment, DevelopmentHosts, RichmondGround};
use urbanization_layer_model::Urbanization;

use crate::cell::world_slot;

/// What Maputo reads from the ground under it.
///
/// World-space slots for each built development. Urbanization's contract does
/// not grow a furniture method for this.
pub trait FurnitureSlots: Send + Sync + 'static {
	/// The shared value one development's slots come from.
	type Development: hcsg::GenerationScheme;

	/// `development`'s world-space High slots.
	fn development_slots(development: &Self::Development) -> Vec<FurnitureNode>;
}

impl<G: RichmondGround> FurnitureSlots for Urbanization<richmond::Richmond<G>> {
	type Development = Built<G>;

	fn development_slots(built: &Built<G>) -> Vec<FurnitureNode> {
		world_slots_of(&built.development)
	}
}

fn world_slots_of(development: &BuiltDevelopment) -> Vec<FurnitureNode> {
	let mut out = Vec::new();
	for host in development.hosts() {
		let transform = host.transform();
		for node in host.furniture_nodes() {
			out.push(world_slot(transform, node));
		}
		for node in expand_usages(host.furniture_usage_nodes()) {
			out.push(world_slot(transform, node));
		}
	}
	out
}
