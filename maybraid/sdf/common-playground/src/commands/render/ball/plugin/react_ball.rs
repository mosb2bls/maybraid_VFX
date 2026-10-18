//! Reacts to [`super::super::BallHelper`].

use bevy::prelude::*;

use crate::commands::render::ball::BallHelper;
use crate::commands::render::preview_from_helper::{
	commit_render_preview, render_helper_preview_config,
};
use crate::preview::PreviewConfig;
use game_commands::command::CommandConsoleOutput;

pub fn react_render_helper_ball(
	mut commands: Commands,
	q: Query<(Entity, &BallHelper), Added<BallHelper>>,
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
