use bevy::prelude::*;
use maybraid_spatial_rumble::{SpatialRumbleOutput, SpatialRumbleSource};

#[derive(Component)]
pub struct StatusText;

pub fn setup_status(mut commands: Commands) {
	commands.spawn((
		Text::new(status_line(0, &SpatialRumbleOutput::default(), false)),
		TextFont { font_size: FontSize::Px(18.0), ..default() },
		TextColor(Color::srgb(0.86, 0.88, 0.82)),
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(16.0),
			top: Val::Px(16.0),
			..default()
		},
		StatusText,
	));
}

pub fn sync_status_text(
	output: Res<SpatialRumbleOutput>,
	time: Res<Time<Virtual>>,
	sources: Query<&SpatialRumbleSource>,
	mut texts: Query<&mut Text, With<StatusText>>,
) {
	let Ok(mut text) = texts.single_mut() else {
		return;
	};
	*text = Text::new(status_line(sources.iter().count(), &output, time.is_paused()));
}

fn status_line(live: usize, output: &SpatialRumbleOutput, paused: bool) -> String {
	format!(
		"spatial rumble  live={live}  camera={:.3}  roll={:.3}  ground={:.3}  pad={:.2}/{:.2}{}",
		output.camera_offset.length(),
		output.camera_roll,
		output.ground.amplitude,
		output.pad_weak,
		output.pad_strong,
		if paused { "  PAUSED" } else { "" }
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn status_mentions_pause() {
		let line = status_line(2, &SpatialRumbleOutput::default(), true);
		assert!(line.contains("live=2"));
		assert!(line.contains("PAUSED"));
	}
}
