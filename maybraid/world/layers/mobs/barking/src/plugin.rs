//! Runtime patch queue for world-model adapters.

use bevy::prelude::*;
use mob_scenes::MobScenesPlugin;

use crate::MobGroup;

#[derive(Resource, Default)]
pub struct PendingMobGroups {
	groups: Vec<MobGroup>,
}

impl PendingMobGroups {
	pub fn push(&mut self, group: MobGroup) {
		self.groups.push(group);
	}

	pub fn is_empty(&self) -> bool {
		self.groups.is_empty()
	}
}

pub struct MobGroupsPlugin;

impl Plugin for MobGroupsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MobScenesPlugin>() {
			app.add_plugins(MobScenesPlugin);
		}
		app.init_resource::<PendingMobGroups>()
			.add_systems(Update, spawn_pending_groups);
	}
}

fn spawn_pending_groups(mut commands: Commands, mut pending: ResMut<PendingMobGroups>) {
	for group in std::mem::take(&mut pending.groups) {
		group.spawn(&mut commands);
	}
}
