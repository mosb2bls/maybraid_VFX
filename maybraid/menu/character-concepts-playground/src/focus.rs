use bevy::prelude::*;
use character_ui_menu::CameraFocus;
use characters::CharacterPartSlot;

use crate::{
	preview::{PreviewAssetTarget, PreviewTarget},
	skinning::CharacterPart,
	ui::CreatorUiState,
};

#[derive(Component, Clone, Copy)]
pub struct PreviewFocusBaseScale(pub Vec3);

pub fn animate_focused_preview_asset(
	mut commands: Commands,
	time: Res<Time>,
	ui_state: Res<CreatorUiState>,
	mut parts: Query<(
		Entity,
		&PreviewAssetTarget,
		&CharacterPart,
		&mut Transform,
		Option<&PreviewFocusBaseScale>,
	)>,
) {
	let focus = ui_state.focused_target();
	for (entity, target, part, mut transform, base) in &mut parts {
		if !should_pulse(part.slot) {
			if let Some(base) = base {
				transform.scale = base.0;
			}
			continue;
		}
		let base_scale = match base {
			Some(base) => base.0,
			None => {
				let scale = transform.scale;
				commands.entity(entity).try_insert(PreviewFocusBaseScale(scale));
				scale
			}
		};
		let pulse = focus_scale(focus, target.target, time.elapsed_secs());
		transform.scale = base_scale * pulse;
	}
}

fn should_pulse(_slot: CharacterPartSlot) -> bool {
	false
}

fn focus_scale(focus: Option<CameraFocus>, target: PreviewTarget, elapsed: f32) -> f32 {
	if focus.is_none() || !focus_matches_preview_target(target) {
		return 1.0;
	}
	1.0 + elapsed.mul_add(7.0, 0.0).sin().abs() * 0.045
}

fn focus_matches_preview_target(_target: PreviewTarget) -> bool {
	true
}
