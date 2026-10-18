//! Discovery: the streamed Maybraid world.
//!
//! The game shell still owns cameras, loading, and pause. This crate is the
//! session those systems ask when the home row enters Discovery.

use std::any::TypeId;

use bevy::ecs::system::SystemParamItem;
use bevy::prelude::*;
use durham::Durham;
use layer_stack::{ActiveGenerationMode, GenerationMode};
use lod::hcsg::{Gated, HcsgGate};
use terrain_layer_model::TerrainStreaming;
use world_player::{ModePlayerPolicies, ModePlayerPolicy};

pub const LABEL: &str = "Discovery";

/// Terrain, vegetation, and urbanization stream while Discovery is in the world shell.
pub fn streams_terrain(session_is_discovery: bool, in_world_shell: bool) -> bool {
	session_is_discovery && in_world_shell
}

/// Playable-world generation.
pub struct Discovery;

impl GenerationMode for Discovery {}

/// Open while Discovery is the active mode and its world streams.
pub struct DiscoveryGate;

impl HcsgGate for DiscoveryGate {
	type Param =
		(Res<'static, TerrainStreaming<Durham>>, Res<'static, State<ActiveGenerationMode>>);

	fn open((streaming, mode): &SystemParamItem<Self::Param>) -> bool {
		streaming.enabled && mode.get().is::<Discovery>()
	}
}

/// `B`'s regions while Discovery streams, and none otherwise.
pub type InDiscovery<B> = Gated<DiscoveryGate, B>;

/// Discovery's player home is the origin. Waypoints stay; first load and death
/// both open the POI picker.
pub struct DiscoveryPlayerPlugin;

impl Plugin for DiscoveryPlayerPlugin {
	fn build(&self, app: &mut App) {
		let mut policies = app.world_mut().get_resource_or_insert_with(ModePlayerPolicies::default);
		policies.register(TypeId::of::<Discovery>(), discovery_player_policy());
	}
}

fn discovery_player_policy() -> ModePlayerPolicy {
	ModePlayerPolicy {
		home: Vec2::ZERO,
		keep_waypoints: true,
		respawn_ends_life: false,
		pick_first_spawn: true,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::{RunSystemOnce, StaticSystemParam};
	use bevy::state::app::StatesPlugin;

	fn gate_with(mode: ActiveGenerationMode, enabled: bool) -> anyhow::Result<bool> {
		let mut app = App::new();
		app.add_plugins(StatesPlugin).insert_state(mode);
		app.insert_resource(TerrainStreaming::<Durham>::new(enabled));
		app.update();
		app.world_mut()
			.run_system_once(|param: StaticSystemParam<<DiscoveryGate as HcsgGate>::Param>| {
				DiscoveryGate::open(&param)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))
	}

	#[test]
	fn the_gate_opens_only_while_discovery_streams() -> anyhow::Result<()> {
		anyhow::ensure!(gate_with(ActiveGenerationMode::of::<Discovery>(), true)?);
		anyhow::ensure!(!gate_with(ActiveGenerationMode::of::<Discovery>(), false)?);
		anyhow::ensure!(!gate_with(ActiveGenerationMode::None, true)?);
		Ok(())
	}

	#[test]
	fn discovery_streams_only_inside_the_world_shell() -> anyhow::Result<()> {
		anyhow::ensure!(streams_terrain(true, true));
		anyhow::ensure!(!streams_terrain(true, false));
		anyhow::ensure!(!streams_terrain(false, true));
		Ok(())
	}
}
