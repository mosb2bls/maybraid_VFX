//! Paint a skill map into a texture, then archive it through screen-capture.

use std::collections::HashSet;

use bevy::prelude::*;
use item_icons::{ItemCaptureSubjects, ITEM_ICON_RESOLUTION};
use screen_capture::{
	CaptureCacheKey, CaptureClient, CaptureId, CaptureMode, CaptureRequest, CaptureRevision,
};

use crate::preview::{spawn_skill_map_paint_view, ICON_PAINT_ID_BASE};
use crate::tile_material::SkillMapTileAssets;
use crate::SkillMapId;

const PAINT_PX: u32 = 256;
const PAINT_SETTLE_FRAMES: u32 = 3;

#[derive(Resource, Default)]
pub(crate) struct SkillMapPaintSlots(u32);

#[derive(Resource, Default)]
pub(crate) struct SkillMapIconSubmitted(HashSet<CaptureId>);

#[derive(Component)]
pub(crate) struct SkillMapIconPaint {
	id: CaptureId,
	stem: String,
	booth: Entity,
	image: Handle<Image>,
	frames: u32,
}

pub(crate) fn realize_skill_map_capture_hosts(
	mut commands: Commands,
	mut slots: ResMut<SkillMapPaintSlots>,
	mut submitted: ResMut<SkillMapIconSubmitted>,
	mut client: CaptureClient,
	subjects: Option<Res<ItemCaptureSubjects>>,
	tiles: Option<Res<SkillMapTileAssets>>,
	mut paints: Query<(Entity, &mut SkillMapIconPaint)>,
) {
	let (Some(subjects), Some(tiles)) = (subjects, tiles) else {
		return;
	};
	let painting_ids: HashSet<CaptureId> =
		paints.iter().map(|(_, paint)| paint.id.clone()).collect();
	for item in subjects.items() {
		let Some(spec) = item.skill_map_spec() else {
			continue;
		};
		let Some(label) = item.capture_id_label() else {
			continue;
		};
		let id = CaptureId::new(label);
		if submitted.0.contains(&id) || painting_ids.contains(&id) {
			continue;
		}
		if cache_has_both(&client, &id, &item.name()) {
			submit_image(&mut client, &id, &item.name(), Handle::default());
			submitted.0.insert(id);
			continue;
		}
		let map_id = SkillMapId(ICON_PAINT_ID_BASE + slots.0);
		slots.0 = slots.0.wrapping_add(1);
		let booth = commands
			.spawn((
				Name::new(format!("skill-map-icon-paint-{}", spec.kind.label())),
				Transform::default(),
				Visibility::default(),
			))
			.id();
		let image = spawn_skill_map_paint_view(
			&mut commands,
			&mut client.images,
			&tiles,
			spec,
			map_id,
			PAINT_PX,
			booth,
		);
		commands.spawn(SkillMapIconPaint { id, stem: item.name(), booth, image, frames: 0 });
	}

	let mut finished = Vec::new();
	for (entity, mut paint) in &mut paints {
		paint.frames = paint.frames.saturating_add(1);
		if paint.frames < PAINT_SETTLE_FRAMES {
			continue;
		}
		submit_image(&mut client, &paint.id, &paint.stem, paint.image.clone());
		submitted.0.insert(paint.id.clone());
		finished.push((entity, paint.booth));
	}
	for (entity, booth) in finished {
		commands.entity(entity).try_despawn();
		commands.entity(booth).try_despawn();
	}
}

fn cache_has_both(client: &CaptureClient, id: &CaptureId, stem: &str) -> bool {
	[CaptureMode::Shaded, CaptureMode::Silhouette].into_iter().all(|mode| {
		let key = CaptureCacheKey::from_request(&image_request(id, Handle::default(), mode, stem));
		client.cached(&key).is_some() || client.cache.cache.load(&key).is_some()
	})
}

fn submit_image(client: &mut CaptureClient, id: &CaptureId, stem: &str, image: Handle<Image>) {
	for mode in [CaptureMode::Shaded, CaptureMode::Silhouette] {
		client.request(image_request(id, image.clone(), mode, stem));
	}
}

fn image_request(
	id: &CaptureId,
	image: Handle<Image>,
	mode: CaptureMode,
	stem: &str,
) -> CaptureRequest {
	CaptureRequest::from_image_handle(
		id.clone(),
		image,
		mode,
		ITEM_ICON_RESOLUTION,
		CaptureRevision::CURRENT,
	)
	.with_file_stem(stem)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn icon_paint_ids_stay_off_the_live_map_layer() {
		assert!(ICON_PAINT_ID_BASE > 0);
		assert_ne!(SkillMapId(ICON_PAINT_ID_BASE).0, 0);
	}

	#[test]
	fn paint_and_capture_params_are_disjoint() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		let mut system = IntoSystem::into_system(realize_skill_map_capture_hosts);
		system.initialize(app.world_mut());
	}
}
