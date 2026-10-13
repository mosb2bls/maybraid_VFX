//! Walk from a nested visual to the plant's [`IntelligenceLod`].

use bevy::prelude::*;
use intelligence_lod::IntelligenceLod;

/// Plant entity that carries [`IntelligenceLod`], walking toward the root.
///
/// Band is on the plant, not the nested body. Missing = Near (caller).
pub fn plant_lod_entity(
	start: Entity,
	child_of: &Query<&ChildOf>,
	lods: &Query<&IntelligenceLod>,
) -> Option<Entity> {
	let mut current = Some(start);
	for _ in 0..32 {
		let Some(entity) = current else {
			break;
		};
		if lods.contains(entity) {
			return Some(entity);
		}
		current = child_of.get(entity).ok().map(ChildOf::parent);
	}
	None
}

/// Plant [`IntelligenceLod`] from a nested visual. Missing = Near.
pub fn plant_lod<'a>(
	start: Entity,
	child_of: &Query<&ChildOf>,
	lods: &'a Query<&IntelligenceLod>,
) -> Option<&'a IntelligenceLod> {
	plant_lod_entity(start, child_of, lods).and_then(|entity| lods.get(entity).ok())
}
