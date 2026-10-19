//! Shared [`PreviewConfig`] construction and commit side effects for render helpers.

use bevy::prelude::*;
use game_commands::command::CommandConsoleOutput;
use sdf_common::{Ball, CrookCylinder, NoisySurface, TaperedCylinder};

use crate::preview::PreviewConfig;
use crate::primitive::PlaygroundPrimitive;

use super::{NoisyBallArgs, NoisyCrookCylinderArgs, NoisyCylinderArgs, RenderHelper};

/// Maps a render helper's inner CLI geometry into a [`PlaygroundPrimitive`].
pub(crate) trait IntoPlaygroundPrimitive {
	fn into_playground_primitive(&self) -> PlaygroundPrimitive;
}

impl IntoPlaygroundPrimitive for TaperedCylinder {
	fn into_playground_primitive(&self) -> PlaygroundPrimitive {
		PlaygroundPrimitive::TaperedCylinder(*self)
	}
}

impl IntoPlaygroundPrimitive for CrookCylinder {
	fn into_playground_primitive(&self) -> PlaygroundPrimitive {
		PlaygroundPrimitive::CrookCylinder(*self)
	}
}

impl IntoPlaygroundPrimitive for Ball {
	fn into_playground_primitive(&self) -> PlaygroundPrimitive {
		PlaygroundPrimitive::Ball(*self)
	}
}

impl IntoPlaygroundPrimitive for NoisyCylinderArgs {
	fn into_playground_primitive(&self) -> PlaygroundPrimitive {
		PlaygroundPrimitive::NoisyCylinder(NoisySurface::from_params(
			self.cylinder,
			self.resolved_noise(),
		))
	}
}

impl IntoPlaygroundPrimitive for NoisyCrookCylinderArgs {
	fn into_playground_primitive(&self) -> PlaygroundPrimitive {
		PlaygroundPrimitive::NoisyCrookCylinder(NoisySurface::from_params(
			self.crook,
			self.resolved_noise(),
		))
	}
}

impl IntoPlaygroundPrimitive for NoisyBallArgs {
	fn into_playground_primitive(&self) -> PlaygroundPrimitive {
		PlaygroundPrimitive::NoisyBall(NoisySurface::from_params(self.ball, self.resolved_noise()))
	}
}

pub(crate) fn render_helper_preview_config<T: IntoPlaygroundPrimitive + clap::Args>(
	helper: &RenderHelper<T>,
) -> PreviewConfig {
	PreviewConfig {
		primitive: helper.inner.into_playground_primitive(),
		res_2: helper.res_2,
		transform: helper.preview_transform(),
	}
}

/// Apply a preview update and tear down the transient helper entity.
pub(crate) fn commit_render_preview(
	commands: &mut Commands,
	entity: Entity,
	config: PreviewConfig,
	preview: &mut PreviewConfig,
	console: &mut CommandConsoleOutput,
) {
	*preview = config;
	console.0.clear();
	commands.entity(entity).despawn();
}
