use std::collections::HashMap;

use bevy::prelude::*;

use crate::{skinning::CharacterPart, thumbnail::ThumbnailPreview};

#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) struct AppliedThumbnailColor(Color);

#[derive(Resource, Default)]
pub struct PreviewColorMaterials {
	thumbnail_handles: HashMap<[u8; 4], Handle<StandardMaterial>>,
}

impl PreviewColorMaterials {
	fn thumbnail_handle(
		&mut self,
		color: Color,
		materials: &mut Assets<StandardMaterial>,
	) -> Handle<StandardMaterial> {
		let key = color_key(color);
		self.thumbnail_handles
			.entry(key)
			.or_insert_with(|| {
				materials.add(StandardMaterial { base_color: color, cull_mode: None, ..default() })
			})
			.clone()
	}
}

pub fn apply_preview_colors(
	mut commands: Commands,
	thumbnail_roots: Query<(Entity, &ThumbnailPreview, Option<&Children>)>,
	character_parts: Query<Entity, With<CharacterPart>>,
	children_q: Query<&Children>,
	mut meshes: Query<(Entity, &mut MeshMaterial3d<StandardMaterial>)>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut color_materials: ResMut<PreviewColorMaterials>,
) {
	for (root, preview, children) in &thumbnail_roots {
		apply_thumbnail_color_to_tree(
			root,
			preview.color,
			children,
			&character_parts,
			&children_q,
			&mut meshes,
			&mut materials,
			&mut color_materials,
			&mut commands,
		);
	}
}

fn apply_thumbnail_color_to_tree(
	root: Entity,
	color: Color,
	children: Option<&Children>,
	character_parts: &Query<Entity, With<CharacterPart>>,
	children_q: &Query<&Children>,
	meshes: &mut Query<(Entity, &mut MeshMaterial3d<StandardMaterial>)>,
	materials: &mut Assets<StandardMaterial>,
	color_materials: &mut PreviewColorMaterials,
	commands: &mut Commands,
) {
	let Some(children) = children else {
		return;
	};
	let handle = color_materials.thumbnail_handle(color, materials);
	let mut stack: Vec<(Entity, bool)> = children.iter().map(|entity| (entity, false)).collect();
	while let Some((entity, is_entry)) = stack.pop() {
		if !is_entry && character_parts.contains(entity) && entity != root {
			continue;
		}
		if let Ok((mesh, mut material)) = meshes.get_mut(entity) {
			material.0 = handle.clone();
			commands.entity(mesh).try_insert(AppliedThumbnailColor(color));
		}
		if let Ok(children) = children_q.get(entity) {
			for child in children.iter() {
				stack.push((child, false));
			}
		}
	}
}

fn color_key(color: Color) -> [u8; 4] {
	let color = color.to_srgba();
	[
		(color.red * 255.0).round() as u8,
		(color.green * 255.0).round() as u8,
		(color.blue * 255.0).round() as u8,
		(color.alpha * 255.0).round() as u8,
	]
}
