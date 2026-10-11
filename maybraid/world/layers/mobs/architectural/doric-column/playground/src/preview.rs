use bevy::prelude::*;
use doric_column::{
	spawn_doric_column_encounter, spawn_doric_column_summon, DoricColumnSpec,
};

#[derive(Component)]
pub struct PreviewRoot;

pub fn spawn_preview(mut commands: Commands) {
	let encounter = DoricColumnSpec::default();
	spawn_doric_column_encounter(
		&mut commands,
		&encounter,
		Transform::from_xyz(-4.0, 0.0, 0.0),
		false,
	);
	let summon = DoricColumnSpec {
		scale: Vec3::splat(0.85),
		..DoricColumnSpec::default()
	};
	spawn_doric_column_summon(
		&mut commands,
		&summon,
		Transform::from_xyz(4.0, 0.0, 0.0),
		false,
	);
	commands.spawn((PreviewRoot, Name::new("preview-root")));
}
