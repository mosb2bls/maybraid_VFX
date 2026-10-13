//! Preview subject sync for `/show`.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use building_components::FurnitureGeometry;
use furniture_assemblies::{filled_slots_scene, FurnitureKitMeshes};
use game_commands::ui::GameCommandStatusText;

use crate::gallery::{gallery_slots, unit_slot};

#[derive(Component)]
pub struct PreviewRoot;

#[derive(Clone, Debug, PartialEq)]
pub enum PreviewSubject {
	None,
	Unit { geometry: FurnitureGeometry, seed: u64 },
	Gallery,
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct PreviewConfig {
	pub subject: PreviewSubject,
	pub transform: Transform,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { subject: PreviewSubject::None, transform: Transform::IDENTITY }
	}
}

impl PreviewConfig {
	pub fn status_label(&self) -> String {
		match self.subject {
			PreviewSubject::None => {
				"preview: (none — `/show bed|chair|chest|counter|food-display|fruit|range|gallery`)"
					.into()
			}
			PreviewSubject::Unit { geometry, seed } => {
				format!("preview: {geometry:?} seed={seed}")
			}
			PreviewSubject::Gallery => "preview: slot gallery (Richmond rooms + extremes)".into(),
		}
	}
}

pub fn present_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	kits: Option<Res<FurnitureKitMeshes>>,
	roots: Query<Entity, With<PreviewRoot>>,
	mut last: Local<Option<(PreviewSubject, Transform)>>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let Some(kits) = kits else {
		return;
	};
	let key = (config.subject.clone(), config.transform);
	let changed = last.as_ref() != Some(&key);
	let has_root = roots.iter().next().is_some();

	if matches!(config.subject, PreviewSubject::None) {
		if changed || has_root {
			for entity in &roots {
				commands.entity(entity).despawn();
			}
			*last = Some(key);
		}
		return;
	}

	if !changed && has_root {
		return;
	}

	for entity in &roots {
		commands.entity(entity).despawn();
	}
	*last = Some(key);

	let nodes = match &config.subject {
		PreviewSubject::None => Vec::new(),
		PreviewSubject::Unit { geometry, seed } => vec![unit_slot(*geometry, *seed)],
		PreviewSubject::Gallery => match gallery_slots() {
			Ok(slots) => slots.into_iter().map(|s| s.node).collect(),
			Err(err) => {
				status.0 = format!("gallery failed: {err}");
				error!("gallery failed: {err}");
				return;
			}
		},
	};

	let transform = config.transform;
	commands
		.spawn_scene((
			filled_slots_scene(&nodes, &kits),
			bsn! {
				template_value(transform)
				Visibility::default()
			},
		))
		.insert(PreviewRoot);
}
