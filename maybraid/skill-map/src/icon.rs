//! Realize a skill-map catalog capture into an isolated [`CaptureHostRoot`].

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use item_icons::{CaptureHostRealized, ItemCaptureSubjects, HOST_SKILL_MAP};
use screen_capture::{CaptureHostReady, CaptureHostRoot, CAPTURE_RENDER_LAYER};

use crate::map::{authored_map_from_spec, SkillMapId};
use crate::tile_material::SkillMapTileAssets;
use crate::tiles::spawn_map_tiles_at;

pub(crate) fn realize_skill_map_capture_hosts(
	mut commands: Commands,
	subjects: Option<Res<ItemCaptureSubjects>>,
	tiles: Option<Res<SkillMapTileAssets>>,
	hosts: Query<(Entity, &CaptureHostRoot), Without<CaptureHostRealized>>,
	children: Query<&Children>,
) {
	let (Some(subjects), Some(tiles)) = (subjects, tiles) else {
		return;
	};
	for (root, host) in &hosts {
		if host.kind != HOST_SKILL_MAP {
			continue;
		}
		let Some(spec) = subjects.get(&host.id).and_then(|item| item.skill_map_spec()) else {
			continue;
		};
		let mut authored = authored_map_from_spec(spec);
		authored.id = SkillMapId(0);
		spawn_map_tiles_at(&mut commands, authored, None, &tiles, Vec2::ZERO, Some(root));
		stamp_capture_layers(&mut commands, root, &children);
		commands.entity(root).try_insert((CaptureHostRealized, CaptureHostReady));
	}
}

fn stamp_capture_layers(commands: &mut Commands, root: Entity, children: &Query<&Children>) {
	let layers = RenderLayers::layer(CAPTURE_RENDER_LAYER);
	visit(root, children, &mut |entity| {
		commands.entity(entity).try_insert(layers.clone());
	});
}

fn visit(root: Entity, children: &Query<&Children>, visit: &mut impl FnMut(Entity)) {
	visit(root);
	let Ok(kids) = children.get(root) else {
		return;
	};
	for child in kids.iter() {
		self::visit(child, children, visit);
	}
}
