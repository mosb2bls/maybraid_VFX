//! Reacts to [`super::super::NoisyCrookCylinderHelper`].

use bevy::prelude::*;

use crate::commands::render::noisy_crook_cylinder::NoisyCrookCylinderHelper;
use crate::commands::render::preview_from_helper::{
	commit_render_preview, render_helper_preview_config,
};
use crate::preview::PreviewConfig;
use game_commands::command::CommandConsoleOutput;

pub fn react_render_helper_noisy_crook_cylinder(
	mut commands: Commands,
	q: Query<(Entity, &NoisyCrookCylinderHelper), Added<NoisyCrookCylinderHelper>>,
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
