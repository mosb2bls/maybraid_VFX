//! Terrain placement contract: ground_contact sits on the sampled surface.

use bevy::prelude::*;
use ground::ElevationProbe;

use crate::anchors::DoricColumnGroundContact;
use crate::spec::DoricColumnSpec;

/// When present, the root's [`DoricColumnGroundContact`] child is snapped to ground each frame.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DoricColumnTerrainPlacement;

const PROBE_LIFT: f32 = 4.0;
const PROBE_DISTANCE: f32 = 40.0;

/// Move the assembly so `ground_contact` rests on the highest standable Fixed hit below it.
pub fn snap_doric_columns_to_terrain<P: ElevationProbe>(
	probe: &mut P,
	columns: Query<(Entity, &mut Transform, &Children), With<DoricColumnTerrainPlacement>>,
	contacts: Query<&GlobalTransform, With<DoricColumnGroundContact>>,
	child_of: Query<&ChildOf>,
) {
	for (root, mut transform, children) in columns {
		let contact = children
			.iter()
			.find_map(|child| contacts.get(child).ok().map(|gt| (child, gt.translation())));
		let Some((contact_entity, world_contact)) = contact else {
			continue;
		};
		let exclude = ancestor_chain(root, contact_entity, &child_of);
		let origin = world_contact + Vec3::Y * PROBE_LIFT;
		let Some(hit) = probe.hit_down(origin, PROBE_DISTANCE + PROBE_LIFT, &exclude) else {
			continue;
		};
		let delta = hit.point.y - world_contact.y;
		if delta.abs() > 1e-5 {
			transform.translation.y += delta;
		}
	}
}

fn ancestor_chain(root: Entity, leaf: Entity, child_of: &Query<&ChildOf>) -> Vec<Entity> {
	let mut chain = vec![root];
	let mut current = Some(leaf);
	for _ in 0..16 {
		let Some(entity) = current else {
			break;
		};
		if entity == root {
			break;
		}
		chain.push(entity);
		current = child_of.get(entity).ok().map(|c| c.parent());
	}
	chain
}

/// Expected ground-contact height for a spec at identity (y = 0).
pub fn ground_contact_local_y(_spec: &DoricColumnSpec) -> f32 {
	0.0
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ground_contact_is_at_assembly_origin() {
		assert_eq!(ground_contact_local_y(&DoricColumnSpec::default()), 0.0);
	}
}
