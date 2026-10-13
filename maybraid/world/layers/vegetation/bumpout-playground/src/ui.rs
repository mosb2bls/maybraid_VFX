use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use vegetation_bumpout::BumpOut;

use crate::{NeighborhoodControls, PresenterLayer, TileCoordinate};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Chico bump-outs - / cmd - L look - WASD - Space up - PgUp/PgDn scroll".into(),
		empty_console_text:
			"Console: `neighborhood …`, `visibility toggle`, `help` — wheel or PgUp/PgDn".into(),
		root_background: Color::srgba(0.1, 0.2, 0.24, 0.82),
		controls_hint: "help — neighborhood … — visibility toggle — Enter — history — PgUp/PgDn"
			.into(),
	}
}

pub(crate) fn sync_command_status_text(
	controls: Res<NeighborhoodControls>,
	layers: Query<(&PresenterLayer, &TileCoordinate, &BumpOut)>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let Some((_, _, bump_out)) = layers
		.iter()
		.find(|(layer, tile, _)| **layer == controls.layer && tile.0 == IVec2::ZERO)
	else {
		status.0 = "Waiting for center bump-out tile…".into();
		return;
	};
	let neighborhood = bump_out.neighborhood();
	let index = controls.sample_index();
	let coordinate = controls.selected_coordinate();
	status.0 = format!(
		"5×5 tiles | editing {} sample ({}, {})\n\
		 density {:.3} | bite-size {:.2} | bite-size-deviation {:.2}\n\
		 average-height {:.2} | height-deviation {:.2}",
		controls.layer.label(),
		coordinate.x,
		coordinate.y,
		neighborhood.densities[index],
		neighborhood.bite_sizes[index],
		neighborhood.bite_size_deviations[index],
		neighborhood.average_heights[index],
		neighborhood.height_deviations[index],
	);
}
