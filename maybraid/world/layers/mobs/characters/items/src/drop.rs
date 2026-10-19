//! Selective bag drop: request, policy, and outcome.
//!
//! [`Inventory::try_drop`] is the UI-agnostic mutation. It removes one bag
//! index, unequips it when it was worn or queued, and remaps every remaining
//! selection so the cycle queues and primary stay valid. Callers that talk to
//! ECS emit [`InventoryDropRequest`]; world adapters persist through
//! [`crate::Inventory`] snapshots and optional [`InventoryDropped`].
//!
//! # Open questions (placeholder policy)
//!
//! These are **not** finalized. [`InventoryDropPolicy::PLACEHOLDER`] is the
//! conservative stand-in so gameplay can ship without locking design:
//!
//! 1. **World pickup.** Default `spawn_world_pickup` is `false` on
//!    [`InventoryDropPolicy::PLACEHOLDER`]. The world host inserts an
//!    [`ActiveInventoryDropPolicy`] with the flag on so live drops can toss a
//!    claimable stash. The adapter is the sole commit: it applies `try_drop`
//!    under the request policy and emits [`InventoryDropped`]. World hosts
//!    spawn a pickup from that outcome when the policy asks for one, using the
//!    accepted owner's transform. Gallery / create stay on the pure bag
//!    mutation and remain delete-only. Tab still explodes the whole bag
//!    through the stash module.
//! 2. **Undroppable items.** Lubikans are not bag items (ledger + charge live
//!    elsewhere). Tanzes are ordinary bag items in the Tanzes slot and follow
//!    this same placeholder policy (equipped drops are allowed; no extra
//!    undroppable flag). Starter items have no durable flag. Default allows
//!    equipped items and an empty leftover bag. Flip
//!    [`InventoryDropPolicy::allow_equipped`] or
//!    [`InventoryDropPolicy::allow_empty_bag`] when those rules exist.
//! 3. **Confirm.** Default `require_confirm` is `true`. The domain does not
//!    prompt; UI reads the flag. In-game pause edits skip the card because
//!    the world pile is recoverable. Gallery / create still confirm.

use bevy::prelude::*;

use crate::{Inventory, InventoryItem, InventorySlot};

/// Conservative stand-in for unresolved drop rules. See module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryDropPolicy {
	/// When true, world adapters may spawn a claimable pickup. The domain
	/// never spawns; it only reports the flag on [`InventoryDrop`].
	pub spawn_world_pickup: bool,
	/// When true, UI should confirm before calling [`Inventory::try_drop`].
	pub require_confirm: bool,
	/// When false, refuse if the index is worn or queued.
	pub allow_equipped: bool,
	/// When false, refuse if this would empty the bag. Stand-in for
	/// "starter / last item is undroppable."
	pub allow_empty_bag: bool,
}

impl InventoryDropPolicy {
	/// Conservative defaults: no world pickup, confirm in UI, equipped items
	/// and an empty leftover bag are allowed.
	pub const PLACEHOLDER: Self = Self {
		spawn_world_pickup: false,
		require_confirm: true,
		allow_equipped: true,
		allow_empty_bag: true,
	};
}

impl Default for InventoryDropPolicy {
	fn default() -> Self {
		Self::PLACEHOLDER
	}
}

/// Always-allow removal used to apply an already-accepted drop to a snapshot.
pub const ACCEPTED_DROP_REMOVAL: InventoryDropPolicy = InventoryDropPolicy {
	spawn_world_pickup: false,
	require_confirm: false,
	allow_equipped: true,
	allow_empty_bag: true,
};

/// Process-wide default policy so menus and world adapters share one knob.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveInventoryDropPolicy(pub InventoryDropPolicy);

impl Default for ActiveInventoryDropPolicy {
	fn default() -> Self {
		Self(InventoryDropPolicy::PLACEHOLDER)
	}
}

/// Ask the owner's bag to drop `index` under `policy`.
///
/// Input / menus write this. The world adapter that runs inventory drop is
/// the sole owner that accepts or refuses. Combat HUD visibility cannot.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryDropRequest {
	pub owner: Entity,
	pub index: usize,
	pub policy: InventoryDropPolicy,
}

impl InventoryDropRequest {
	pub fn new(owner: Entity, index: usize) -> Self {
		Self { owner, index, policy: InventoryDropPolicy::PLACEHOLDER }
	}

	pub fn with_policy(mut self, policy: InventoryDropPolicy) -> Self {
		self.policy = policy;
		self
	}
}

/// Why [`Inventory::try_drop`] / [`InventoryDropRequest`] was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryDropError {
	MissingIndex,
	PolicyForbidsEquipped,
	PolicyForbidsEmptyBag,
}

impl std::fmt::Display for InventoryDropError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::MissingIndex => write!(f, "bag index is missing"),
			Self::PolicyForbidsEquipped => write!(f, "policy forbids dropping an equipped item"),
			Self::PolicyForbidsEmptyBag => write!(f, "policy forbids emptying the bag"),
		}
	}
}

impl std::error::Error for InventoryDropError {}

/// One accepted bag removal. Selection vectors on the leftover bag are
/// already remapped.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryDrop {
	pub item: InventoryItem,
	/// Caller-facing bag index before the removal.
	pub index: usize,
	pub slot: InventorySlot,
	/// Worn or queued before the drop.
	pub was_selected: bool,
	/// Slot queue head (`weapons[0]` / `skills[0]` / first worn) before the drop.
	pub was_primary: bool,
	pub spawn_world_pickup: bool,
}

/// Accepted drop. Exactly one world adapter should persist / spawn from this.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct InventoryDropped {
	pub owner: Entity,
	pub bag: Entity,
	pub outcome: InventoryDrop,
}

/// Confirmed drop from a menu/editor bag copy. The leftover bag is already
/// updated; world hosts may spawn a claimable pickup from [`Self::item`].
#[derive(Message, Clone, Debug, PartialEq)]
pub struct InventoryMenuDropped {
	pub item: InventoryItem,
	pub index: usize,
}

/// Request owner refused. The bag was not mutated.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryDropRefused {
	pub owner: Entity,
	pub index: usize,
	pub reason: InventoryDropError,
}

impl Inventory {
	/// Whether [`Self::try_drop`] would succeed for `index` under `policy`.
	pub fn can_drop(
		&self,
		index: usize,
		policy: InventoryDropPolicy,
	) -> Result<(), InventoryDropError> {
		let Some(item) = self.items.get(index) else {
			return Err(InventoryDropError::MissingIndex);
		};
		let selected = self.selected(item.slot());
		if selected.contains(&index) && !policy.allow_equipped {
			return Err(InventoryDropError::PolicyForbidsEquipped);
		}
		if self.items.len() == 1 && !policy.allow_empty_bag {
			return Err(InventoryDropError::PolicyForbidsEmptyBag);
		}
		Ok(())
	}

	/// Remove `index`, unequip it when selected, and compact every queue.
	///
	/// Remaining `clothing` / `weapons` / `skills` indices are remapped. A
	/// dropped primary leaves the next queued item as the new head, so Y /
	/// D-pad cycling keeps a valid active selection without combat-hud.
	pub fn try_drop(
		&mut self,
		index: usize,
		policy: InventoryDropPolicy,
	) -> Result<InventoryDrop, InventoryDropError> {
		self.can_drop(index, policy)?;
		let slot = self.items[index].slot();
		let selected = self.selected(slot);
		let was_selected = selected.contains(&index);
		let was_primary = selected.first() == Some(&index);
		let taken = self.take_indices(&[index]);
		let item = taken.items.into_iter().next().ok_or(InventoryDropError::MissingIndex)?;
		Ok(InventoryDrop {
			item,
			index,
			slot,
			was_selected,
			was_primary,
			spawn_world_pickup: policy.spawn_world_pickup,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor, SkillMapKind,
		SkillMapSpec,
	};

	fn mixed_queue() -> Inventory {
		Inventory {
			items: vec![
				InventoryItem::clothing(
					ClothingMesh::Pants,
					ClothingMaterial::Cloth,
					ItemColor::Natural,
				),
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::firearm(FirearmMesh::Reltor),
				InventoryItem::firearm(FirearmMesh::Snailer),
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
			],
			clothing: vec![0],
			weapons: vec![1, 2, 3],
			skills: vec![4, 5],
			tanzes: Vec::new(),
		}
	}

	#[test]
	fn drop_removes_an_unequipped_item_and_remaps_queues() -> anyhow::Result<()> {
		let mut bag = mixed_queue();
		let extra =
			InventoryItem::clothing(ClothingMesh::Robe, ClothingMaterial::Cloth, ItemColor::Cool);
		bag.items.push(extra);
		let outcome = bag.try_drop(6, InventoryDropPolicy::PLACEHOLDER)?;
		assert_eq!(outcome.item.mesh(), Some(ClothingMesh::Robe));
		assert!(!outcome.was_selected);
		assert_eq!(bag.items.len(), 6);
		assert_eq!(bag.clothing, vec![0]);
		assert_eq!(bag.weapons, vec![1, 2, 3]);
		assert_eq!(bag.skills, vec![4, 5]);
		Ok(())
	}

	#[test]
	fn drop_unequips_worn_clothing_and_compacts_later_indices() -> anyhow::Result<()> {
		let mut bag = mixed_queue();
		assert!(bag.toggle(0));
		assert!(bag.clothing.is_empty());
		assert!(bag.toggle(0));
		let outcome = bag.try_drop(0, InventoryDropPolicy::PLACEHOLDER)?;
		assert!(outcome.was_selected);
		assert!(outcome.was_primary);
		assert_eq!(outcome.slot, InventorySlot::Clothing);
		assert!(bag.clothing.is_empty());
		assert_eq!(bag.weapons, vec![0, 1, 2]);
		assert_eq!(bag.skills, vec![3, 4]);
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		Ok(())
	}

	#[test]
	fn drop_primary_weapon_promotes_the_next_queued_gun() -> anyhow::Result<()> {
		let mut bag = mixed_queue();
		let outcome = bag.try_drop(1, InventoryDropPolicy::PLACEHOLDER)?;
		assert!(outcome.was_selected);
		assert!(outcome.was_primary);
		assert_eq!(outcome.slot, InventorySlot::Weapons);
		assert_eq!(bag.weapons, vec![1, 2]);
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		assert_eq!(bag.clothing, vec![0]);
		assert_eq!(bag.skills, vec![3, 4]);
		Ok(())
	}

	#[test]
	fn drop_mid_queue_weapon_keeps_primary_and_compacts_rank() -> anyhow::Result<()> {
		let mut bag = mixed_queue();
		let outcome = bag.try_drop(2, InventoryDropPolicy::PLACEHOLDER)?;
		assert!(outcome.was_selected);
		assert!(!outcome.was_primary);
		assert_eq!(bag.weapons, vec![1, 2]);
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		assert_eq!(bag.rank(2), Some(2));
		Ok(())
	}

	#[test]
	fn drop_primary_skill_map_promotes_the_next_queued_map() -> anyhow::Result<()> {
		let mut bag = mixed_queue();
		let outcome = bag.try_drop(4, InventoryDropPolicy::PLACEHOLDER)?;
		assert!(outcome.was_primary);
		assert_eq!(bag.skills, vec![4]);
		assert_eq!(
			bag.primary_skill_map().and_then(InventoryItem::skill_map_spec),
			Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))
		);
		Ok(())
	}

	#[test]
	fn missing_index_is_refused() {
		let mut bag = mixed_queue();
		assert_eq!(
			bag.try_drop(99, InventoryDropPolicy::PLACEHOLDER).err(),
			Some(InventoryDropError::MissingIndex)
		);
		assert_eq!(bag.items.len(), 6);
		assert_eq!(bag.weapons, vec![1, 2, 3]);
	}

	#[test]
	fn policy_can_refuse_equipped_and_empty_bag() -> anyhow::Result<()> {
		let mut bag = mixed_queue();
		let no_equipped =
			InventoryDropPolicy { allow_equipped: false, ..InventoryDropPolicy::PLACEHOLDER };
		assert_eq!(
			bag.try_drop(1, no_equipped).err(),
			Some(InventoryDropError::PolicyForbidsEquipped)
		);
		assert_eq!(bag.weapons, vec![1, 2, 3]);
		assert!(bag.try_drop(1, InventoryDropPolicy::PLACEHOLDER).is_ok());

		let mut last = Inventory {
			items: vec![InventoryItem::firearm(FirearmMesh::Bullpup)],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		};
		let keep_last =
			InventoryDropPolicy { allow_empty_bag: false, ..InventoryDropPolicy::PLACEHOLDER };
		assert_eq!(
			last.try_drop(0, keep_last).err(),
			Some(InventoryDropError::PolicyForbidsEmptyBag)
		);
		let dropped = last.try_drop(0, InventoryDropPolicy::PLACEHOLDER)?;
		assert!(dropped.was_primary);
		assert!(last.items.is_empty());
		assert!(last.weapons.is_empty());
		Ok(())
	}

	#[test]
	fn placeholder_does_not_ask_the_domain_to_spawn() -> anyhow::Result<()> {
		let mut bag = mixed_queue();
		let outcome = bag.try_drop(3, InventoryDropPolicy::PLACEHOLDER)?;
		assert!(!outcome.spawn_world_pickup);
		Ok(())
	}
}
