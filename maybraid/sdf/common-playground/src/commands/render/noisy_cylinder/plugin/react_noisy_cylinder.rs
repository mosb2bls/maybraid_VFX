//! Reacts to [`super::super::NoisyCylinderHelper`].

use bevy::prelude::*;

use crate::commands::render::noisy_cylinder::NoisyCylinderHelper;
use crate::commands::render::preview_from_helper::{
	commit_render_preview, render_helper_preview_config,
};
use crate::preview::PreviewConfig;
use game_commands::command::CommandConsoleOutput;

pub fn react_render_helper_noisy_cylinder(
	mut commands: Commands,
	q: Query<(Entity, &NoisyCylinderHelper), Added<NoisyCylinderHelper>>,
	mut preview: ResMut<PreviewConfig>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, helper) in &q {
		commit_render_preview(
			&mut commands,
			entity,
			render_helper_preview_config(helper),
			&mut preview,
			&mut console,
		);
	}
}
