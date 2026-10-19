//! Owned items a character can wear or carry.
//!
//! The bag is a flat [`InventoryItem`] list. Each item belongs to one
//! [`InventorySlot`]; that slot holds an ordered unique selection of bag
//! indices (clothing wear order, weapons switch queue, skill-map queue).

use bevy::prelude::*;
use procedural_common::Fnv1a64;

use crate::{
	ClothingKind, ClothingMaterial, ClothingMesh, ClothingStats, FirearmMesh, FirearmSpec,
	FirearmStats, GrenadeRecharge, GrenadeSpec, GrenadeStats, ItemColor, SkillMapKind,
	SkillMapSpec,
};

/// How many garments character creation rolls before the body editor.
pub const STARTER_CLOTHING_COUNT: usize = 3;

/// Capture-id label for the always-equipped Self Lubikan catalog tile.
///
/// Lubikans are not bag items; this label only keys the scene-captured icon.
pub const SELF_LUBIKAN_CAPTURE_LABEL: &str = "self-lubikan";

/// How many firearms character creation rolls into the bag.
pub const STARTER_WEAPON_COUNT: usize = 2;

/// How many grenades character creation adds beside the starter firearms.
pub const STARTER_GRENADE_COUNT: usize = 1;

/// Hard cap on simultaneously worn clothing items.
pub const WORN_CLOTHING_LIMIT: usize = 6;

/// Hard cap on the active weapon queue. Index 0 is the primary.
pub const WEAPON_QUEUE_LIMIT: usize = 3;

/// How many skill maps character creation rolls into the bag.
pub const STARTER_SKILL_MAP_COUNT: usize = 1;

/// Hard cap on the equipped skill-map queue. Index 0 is the presented map.
pub const SKILL_MAP_QUEUE_LIMIT: usize = 3;

/// Hard cap on skill maps sitting in the bag (equipped or not).
pub const SKILL_MAP_BAG_LIMIT: usize = 6;

/// Bag partition. Slots are typed; items are not stored in separate vecs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InventorySlot {
	Clothing,
	Weapons,
	Skills,
}

impl InventorySlot {
	pub const fn capacity(self) -> usize {
		match self {
			Self::Clothing => WORN_CLOTHING_LIMIT,
			Self::Weapons => WEAPON_QUEUE_LIMIT,
			Self::Skills => SKILL_MAP_QUEUE_LIMIT,
		}
	}

	pub const fn label(self) -> &'static str {
		match self {
			Self::Clothing => "Clothing",
			Self::Weapons => "Weapons",
			Self::Skills => "Skill Maps",
		}
	}
}

/// How much of a bag becomes world loot: `numerator / denominator` of the
/// items, with a remainder roll so the expectation is exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LootFraction {
	pub numerator: u32,
	pub denominator: u32,
}

impl LootFraction {
	pub const NONE: Self = Self { numerator: 0, denominator: 1 };
	pub const ALL: Self = Self { numerator: 1, denominator: 1 };
	pub const ONE_THIRD: Self = Self { numerator: 1, denominator: 3 };
	pub const ONE_TWELFTH: Self = Self { numerator: 1, denominator: 12 };

	pub const fn new(numerator: u32, denominator: u32) -> Self {
		Self { numerator, denominator }
	}

	/// How many of `n` items this fraction keeps.
	pub fn count(self, n: usize, rng: &mut ItemRng) -> usize {
		rng.sample_fraction_count(n, self.numerator, self.denominator)
	}
}

/// Recipe name plus palette[0] used to rebuild a clothing [`material_ref::MaterialRef`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialRefParams {
	pub id: ClothingMaterial,
	pub color: ItemColor,
}

impl MaterialRefParams {
	pub const fn new(id: ClothingMaterial, color: ItemColor) -> Self {
		Self { id, color }
	}
}

/// One owned instance in a character inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InventoryItem {
	Clothing { mesh: ClothingMesh, material: MaterialRefParams, stats: ClothingStats },
	Firearm { spec: FirearmSpec, stats: FirearmStats },
	Grenade { spec: GrenadeSpec, stats: GrenadeStats, recharge: GrenadeRecharge },
	SkillMap { spec: SkillMapSpec },
}

impl InventoryItem {
	pub fn clothing(mesh: ClothingMesh, material: ClothingMaterial, color: ItemColor) -> Self {
		Self::Clothing {
			mesh,
			material: MaterialRefParams::new(material, color),
			stats: ClothingStats::generate(mesh, material, color),
		}
	}

	pub fn firearm(mesh: FirearmMesh) -> Self {
		Self::from_firearm_spec(FirearmSpec::from_mesh(mesh))
	}

	pub fn from_firearm_spec(spec: FirearmSpec) -> Self {
		Self::Firearm { spec, stats: FirearmStats::generate(&spec) }
	}

	pub const fn skill_map(spec: SkillMapSpec) -> Self {
		Self::SkillMap { spec }
	}

	pub const fn grenade(spec: GrenadeSpec, stats: GrenadeStats) -> Self {
		Self::Grenade { spec, stats, recharge: GrenadeRecharge { remaining: 0.0 } }
	}

	pub const fn standard_grenade() -> Self {
		Self::grenade(GrenadeSpec::standard(), GrenadeStats::standard())
	}

	pub fn rolled_grenade(rng: &mut ItemRng) -> Self {
		Self::grenade(GrenadeSpec::standard(), GrenadeStats::realize(rng))
	}

	pub const fn slot(&self) -> InventorySlot {
		match self {
			Self::Clothing { .. } => InventorySlot::Clothing,
			Self::Firearm { .. } | Self::Grenade { .. } => InventorySlot::Weapons,
			Self::SkillMap { .. } => InventorySlot::Skills,
		}
	}

	pub const fn mesh(&self) -> Option<ClothingMesh> {
		match self {
			Self::Clothing { mesh, .. } => Some(*mesh),
			Self::Firearm { .. } | Self::Grenade { .. } | Self::SkillMap { .. } => None,
		}
	}

	pub const fn firearm_mesh(&self) -> Option<FirearmMesh> {
		match self {
			Self::Firearm { spec, .. } => Some(spec.kit.body),
			Self::Clothing { .. } | Self::Grenade { .. } | Self::SkillMap { .. } => None,
		}
	}

	pub const fn firearm_spec(&self) -> Option<FirearmSpec> {
		match self {
			Self::Firearm { spec, .. } => Some(*spec),
			Self::Clothing { .. } | Self::Grenade { .. } | Self::SkillMap { .. } => None,
		}
	}

	pub const fn skill_map_spec(&self) -> Option<SkillMapSpec> {
		match self {
			Self::SkillMap { spec } => Some(*spec),
			Self::Clothing { .. } | Self::Firearm { .. } | Self::Grenade { .. } => None,
		}
	}

	pub const fn material(&self) -> Option<MaterialRefParams> {
		match self {
			Self::Clothing { material, .. } => Some(*material),
			Self::Firearm { .. } | Self::Grenade { .. } | Self::SkillMap { .. } => None,
		}
	}

	pub const fn clothing_stats(&self) -> Option<ClothingStats> {
		match self {
			Self::Clothing { stats, .. } => Some(*stats),
			Self::Firearm { .. } | Self::Grenade { .. } | Self::SkillMap { .. } => None,
		}
	}

	pub const fn firearm_stats(&self) -> Option<FirearmStats> {
		match self {
			Self::Firearm { stats, .. } => Some(*stats),
			Self::Clothing { .. } | Self::Grenade { .. } | Self::SkillMap { .. } => None,
		}
	}

	pub const fn grenade_stats(&self) -> Option<GrenadeStats> {
		match self {
			Self::Grenade { stats, .. } => Some(*stats),
			Self::Clothing { .. } | Self::Firearm { .. } | Self::SkillMap { .. } => None,
		}
	}

	pub const fn grenade_spec(&self) -> Option<GrenadeSpec> {
		match self {
			Self::Grenade { spec, .. } => Some(*spec),
			Self::Clothing { .. } | Self::Firearm { .. } | Self::SkillMap { .. } => None,
		}
	}

	pub fn catalog_detail(&self) -> String {
		match self {
			Self::Clothing { stats, .. } => stats.catalog_detail(),
			Self::Firearm { stats, .. } => stats.catalog_detail(),
			Self::Grenade { stats, .. } => stats.catalog_detail(),
			Self::SkillMap { spec } => {
				format!("{} · {:04X}", spec.kind.display_name(), spec.seed as u16)
			}
		}
	}

	pub fn stat_rows(&self) -> Vec<(String, String)> {
		match self {
			Self::Clothing { stats, .. } => stats.stat_rows(),
			Self::Firearm { stats, .. } => stats.stat_rows(),
			Self::Grenade { stats, .. } => stats.stat_rows(),
			Self::SkillMap { spec } => vec![
				(String::from("Kind"), spec.kind.display_name().to_string()),
				(String::from("Seed"), format!("{:08X}", spec.seed)),
			],
		}
	}

	pub const fn label(&self) -> &'static str {
		match self {
			Self::Clothing { mesh, .. } => mesh.label(),
			Self::Firearm { spec, .. } => spec.kit.body.label(),
			Self::Grenade { spec, .. } => spec.mesh.label(),
			Self::SkillMap { spec } => spec.kind.label(),
		}
	}

	pub fn name(&self) -> String {
		match self {
			Self::Clothing { mesh, material, .. } => mesh.hashed_name(material.id, material.color),
			Self::Firearm { spec, .. } => spec.hashed_name(),
			Self::Grenade { spec, .. } => spec.hashed_name(),
			Self::SkillMap { spec } => spec.hashed_name(),
		}
	}

	pub const fn path(&self) -> &'static str {
		match self {
			Self::Clothing { mesh, .. } => mesh.path(),
			Self::Firearm { spec, .. } => spec.kit.body.path(),
			Self::Grenade { .. } | Self::SkillMap { .. } => "",
		}
	}

	/// Stable screen-capture identity.
	pub fn capture_id_label(&self) -> Option<String> {
		match self {
			Self::Firearm { spec, .. } => Some(format!("firearm:{}", spec.identity_label())),
			Self::Clothing { mesh, material, .. } => Some(format!(
				"clothing:{}:{}:{}",
				mesh.label(),
				material.id.label(),
				material.color.label()
			)),
			Self::Grenade { spec, .. } => Some(format!("grenade:{}", spec.mesh.label())),
			Self::SkillMap { spec } => {
				Some(format!("skill-map:{}:{}", spec.kind.label(), spec.seed))
			}
		}
	}

	/// Catalog thumbnail key in a namespace that does not collide with skill maps.
	pub fn icon_catalog_key(&self) -> Option<u64> {
		Some(icon_catalog_key_for_label(&self.capture_id_label()?))
	}

	/// Authored identity: spec/stats/kind match, excluding transient grenade recharge.
	pub fn same_catalog_identity(&self, other: &Self) -> bool {
		match (self, other) {
			(
				Self::Clothing { mesh, material, stats },
				Self::Clothing { mesh: other_mesh, material: other_material, stats: other_stats },
			) => mesh == other_mesh && material == other_material && stats == other_stats,
			(
				Self::Firearm { spec, stats },
				Self::Firearm { spec: other_spec, stats: other_stats },
			) => spec == other_spec && stats == other_stats,
			(
				Self::Grenade { spec, stats, .. },
				Self::Grenade { spec: other_spec, stats: other_stats, .. },
			) => spec == other_spec && stats == other_stats,
			(Self::SkillMap { spec }, Self::SkillMap { spec: other_spec }) => spec == other_spec,
			_ => false,
		}
	}
}

/// Owned items plus per-slot selections. Clothing is wear order; weapons and
/// skill maps are switch queues (first selected is primary / presented).
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory {
	pub items: Vec<InventoryItem>,
	pub clothing: Vec<usize>,
	pub weapons: Vec<usize>,
	pub skills: Vec<usize>,
}

impl Inventory {
	/// Bag layout matches and every item has the same authored identity.
	pub fn same_catalog_identity(&self, other: &Self) -> bool {
		self.clothing == other.clothing
			&& self.weapons == other.weapons
			&& self.skills == other.skills
			&& self.items.len() == other.items.len()
			&& self
				.items
				.iter()
				.zip(&other.items)
				.all(|(left, right)| left.same_catalog_identity(right))
	}

	/// Every clothing item starts worn (up to the wear cap). Weapons stay unequipped.
	pub fn with_all_worn(items: Vec<InventoryItem>) -> Self {
		let clothing: Vec<usize> = items
			.iter()
			.enumerate()
			.filter(|(_, item)| item.slot() == InventorySlot::Clothing)
			.map(|(index, _)| index)
			.take(InventorySlot::Clothing.capacity())
			.collect();
		Self { items, clothing, weapons: Vec::new(), skills: Vec::new() }
	}

	/// Create-mode bag: wear the first lower and first upper; queue every
	/// rolled firearm (up to the weapon cap). Extra clothing stays in the bag.
	pub fn with_starter_outfit(items: Vec<InventoryItem>) -> Self {
		let mut clothing = Vec::new();
		if let Some(index) = items
			.iter()
			.position(|item| item.mesh().is_some_and(|mesh| mesh.kind() == ClothingKind::Lower))
		{
			clothing.push(index);
		}
		if let Some(index) = items
			.iter()
			.position(|item| item.mesh().is_some_and(|mesh| mesh.kind() == ClothingKind::Upper))
		{
			clothing.push(index);
		}
		let weapons: Vec<usize> = items
			.iter()
			.enumerate()
			.filter(|(_, item)| item.slot() == InventorySlot::Weapons)
			.map(|(index, _)| index)
			.take(InventorySlot::Weapons.capacity())
			.collect();
		let skills: Vec<usize> = items
			.iter()
			.enumerate()
			.filter(|(_, item)| item.slot() == InventorySlot::Skills)
			.map(|(index, _)| index)
			.take(InventorySlot::Skills.capacity())
			.collect();
		Self { items, clothing, weapons, skills }
	}

	pub fn selected(&self, slot: InventorySlot) -> &[usize] {
		match slot {
			InventorySlot::Clothing => &self.clothing,
			InventorySlot::Weapons => &self.weapons,
			InventorySlot::Skills => &self.skills,
		}
	}

	fn selected_mut(&mut self, slot: InventorySlot) -> &mut Vec<usize> {
		match slot {
			InventorySlot::Clothing => &mut self.clothing,
			InventorySlot::Weapons => &mut self.weapons,
			InventorySlot::Skills => &mut self.skills,
		}
	}

	/// Clothing wear list (compat with the old `worn` field).
	pub fn worn(&self) -> &[usize] {
		&self.clothing
	}

	pub fn worn_items(&self) -> impl Iterator<Item = &InventoryItem> {
		self.clothing.iter().filter_map(|&index| self.items.get(index))
	}

	pub fn is_worn(&self, index: usize) -> bool {
		self.clothing.contains(&index)
	}

	/// 1-based rank in the item's slot, if selected.
	pub fn rank(&self, index: usize) -> Option<u8> {
		let item = self.items.get(index)?;
		self.selected(item.slot())
			.iter()
			.position(|&selected| selected == index)
			.map(|position| (position + 1) as u8)
	}

	pub fn primary_weapon(&self) -> Option<&InventoryItem> {
		self.weapons.first().and_then(|&index| self.items.get(index))
	}

	pub fn primary_weapon_mut(&mut self) -> Option<&mut InventoryItem> {
		let index = *self.weapons.first()?;
		self.items.get_mut(index)
	}

	pub fn primary_skill_map(&self) -> Option<&InventoryItem> {
		self.skills.first().and_then(|&index| self.items.get(index))
	}

	/// Rotate the equipped skill-map queue. `dir >= 0` advances (D-Pad right);
	/// `dir < 0` goes back. No-op when fewer than two maps are equipped.
	pub fn cycle_skills(&mut self, dir: i8) -> bool {
		if self.skills.len() < 2 {
			return false;
		}
		if dir < 0 {
			self.skills.rotate_right(1);
		} else {
			self.skills.rotate_left(1);
		}
		true
	}

	/// Rotate the switch queue so the next gun becomes primary.
	///
	/// No-op when fewer than two weapons are queued. Returns whether the
	/// primary changed.
	pub fn swap_active(&mut self) -> bool {
		self.cycle_weapons(1)
	}

	/// Rotate the equipped weapon queue. `dir >= 0` advances; `dir < 0` goes back.
	pub fn cycle_weapons(&mut self, dir: i8) -> bool {
		if self.weapons.len() < 2 {
			return false;
		}
		if dir < 0 {
			self.weapons.rotate_right(1);
		} else {
			self.weapons.rotate_left(1);
		}
		true
	}

	/// Wear / queue or remove `index` in its slot. At capacity, selecting a
	/// new item is a no-op. Returns whether the slot changed.
	pub fn toggle(&mut self, index: usize) -> bool {
		let Some(item) = self.items.get(index) else {
			return false;
		};
		let slot = item.slot();
		let selected = self.selected_mut(slot);
		if let Some(position) = selected.iter().position(|&selected| selected == index) {
			selected.remove(position);
			return true;
		}
		if selected.len() >= slot.capacity() {
			return false;
		}
		selected.push(index);
		true
	}

	/// Clothing-only name for [`Self::toggle`].
	pub fn toggle_worn(&mut self, index: usize) -> bool {
		self.toggle(index)
	}

	/// Drain every item and clear wear / queue selections on `self`.
	///
	/// The returned bag keeps the old selection indices so a world stash can
	/// still show toggled clothing and queued weapons. Those indices stay
	/// valid because they refer to the drained `items` list.
	pub fn take_all(&mut self) -> Self {
		std::mem::take(self)
	}

	/// Drain a random subset of bag items as world loot.
	///
	/// Expected count is `items.len() * fraction`. A remainder roll keeps the
	/// expectation exact (four items at 1/3 yield one, plus a 1-in-3 chance
	/// of a second). Source selections that remain are remapped. The leftover
	/// bag keeps the items that were not taken.
	pub fn take_fraction(&mut self, rng: &mut ItemRng, fraction: LootFraction) -> Self {
		let count = fraction.count(self.items.len(), rng);
		self.take_count(rng, count)
	}

	fn take_count(&mut self, rng: &mut ItemRng, count: usize) -> Self {
		if count == 0 || self.items.is_empty() {
			return Self::default();
		}
		if count >= self.items.len() {
			return self.take_all();
		}
		let mut keep = rng.choose_indices(self.items.len(), count);
		keep.sort_unstable();
		self.take_indices(&keep)
	}

	pub(crate) fn take_indices(&mut self, keep: &[usize]) -> Self {
		let len = self.items.len();
		let is_kept = |index: usize| keep.binary_search(&index).is_ok();
		let clothing = std::mem::take(&mut self.clothing);
		let weapons = std::mem::take(&mut self.weapons);
		let skills = std::mem::take(&mut self.skills);
		let mut taken_items = Vec::with_capacity(keep.len());
		let mut remain_items = Vec::with_capacity(len.saturating_sub(keep.len()));
		let mut taken_remap = vec![None; len];
		let mut remain_remap = vec![None; len];
		for (old, item) in std::mem::take(&mut self.items).into_iter().enumerate() {
			if is_kept(old) {
				taken_remap[old] = Some(taken_items.len());
				taken_items.push(item);
			} else {
				remain_remap[old] = Some(remain_items.len());
				remain_items.push(item);
			}
		}
		self.items = remain_items;
		self.clothing = remap_selected(&clothing, &remain_remap);
		self.weapons = remap_selected(&weapons, &remain_remap);
		self.skills = remap_selected(&skills, &remain_remap);
		Self {
			items: taken_items,
			clothing: remap_selected(&clothing, &taken_remap),
			weapons: remap_selected(&weapons, &taken_remap),
			skills: remap_selected(&skills, &taken_remap),
		}
	}

	/// Append `other.items` without auto-wearing or auto-queuing.
	///
	/// Incoming clothing / weapon selections are dropped. Existing selections
	/// on `self` stay valid because new items are only appended.
	pub fn absorb(&mut self, other: Self) {
		self.items.extend(other.items);
	}

	/// One bag per item so each piece can sit on the ground as its own stash.
	///
	/// Each bag visualizes its item (toggled) so clothing and weapons both
	/// render. Wear / queue from the source bag is not preserved — exploded
	/// loot is claimable individually, not worn on pickup.
	pub fn explode(self) -> Vec<Self> {
		self.items
			.into_iter()
			.map(|item| {
				let mut bag = Self {
					items: vec![item],
					clothing: Vec::new(),
					weapons: Vec::new(),
					skills: Vec::new(),
				};
				let _ = bag.toggle(0);
				bag
			})
			.collect()
	}

	pub fn character_sheet(&self) -> crate::CharacterSheet {
		crate::CharacterSheet::from_inventory(self)
	}
}

/// Tiny xorshift64* so this crate does not take on `rand`.
#[derive(Clone, Debug)]
pub struct ItemRng(u64);

impl ItemRng {
	pub const fn from_seed(seed: u64) -> Self {
		Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
	}

	pub fn from_entropy() -> Self {
		let nanos = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|elapsed| elapsed.as_nanos() as u64)
			.unwrap_or(0x9E37_79B9_7F4A_7C15);
		Self::from_seed(nanos.wrapping_mul(0xA24B_AED4_96E9_C13F) ^ 0xD1B5_4A32_D192_ED03)
	}

	fn next_u64(&mut self) -> u64 {
		let mut x = self.0;
		x ^= x >> 12;
		x ^= x << 25;
		x ^= x >> 27;
		self.0 = x;
		x.wrapping_mul(0x2545_F491_4F6C_DD1D)
	}

	pub fn gen_index(&mut self, len: usize) -> usize {
		if len <= 1 {
			return 0;
		}
		(self.next_u64() as usize) % len
	}

	/// Inclusive range. `max < min` returns `min`.
	pub fn in_range(&mut self, min: u32, max: u32) -> u32 {
		if max <= min {
			return min;
		}
		min + (self.next_u64() % (u64::from(max) - u64::from(min) + 1)) as u32
	}

	pub fn in_range_i16(&mut self, min: i16, max: i16) -> i16 {
		if max <= min {
			return min;
		}
		let span = (i32::from(max) - i32::from(min) + 1) as u32;
		min.saturating_add(self.in_range(0, span - 1) as i16)
	}

	/// Uniform in `[0, 1)`.
	pub fn unit(&mut self) -> f32 {
		(self.next_u64() >> 11) as f32 / ((1u64 << 53) as f32)
	}

	/// Box–Muller sample of \(\mathcal{N}(\mu, \sigma)\).
	pub fn sample_normal(&mut self, mean: f32, sd: f32) -> f32 {
		let u1 = self.unit().max(1e-7);
		let u2 = self.unit();
		let z = (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos();
		mean + sd * z
	}

	pub fn choose<'a, T>(&mut self, values: &'a [T]) -> Option<&'a T> {
		if values.is_empty() {
			return None;
		}
		Some(&values[self.gen_index(values.len())])
	}

	/// How many of `n` items a `numerator / denominator` drop keeps.
	///
	/// Remainder is an extra Bernoulli trial so expectation is exact.
	pub fn sample_fraction_count(&mut self, n: usize, numerator: u32, denominator: u32) -> usize {
		if n == 0 || numerator == 0 || denominator == 0 {
			return 0;
		}
		if numerator >= denominator {
			return n;
		}
		let product = n as u64 * u64::from(numerator);
		let den = u64::from(denominator);
		let base = (product / den) as usize;
		let rem = (product % den) as u32;
		if rem > 0 && self.in_range(0, denominator - 1) < rem {
			base + 1
		} else {
			base
		}
	}

	/// `count` distinct indices in `0..len`, in shuffled order.
	pub fn choose_indices(&mut self, len: usize, count: usize) -> Vec<usize> {
		let count = count.min(len);
		let mut indices: Vec<usize> = (0..len).collect();
		for index in 0..count {
			let swap = index + self.gen_index(len - index);
			indices.swap(index, swap);
		}
		indices.truncate(count);
		indices
	}

	/// One random clothing item (mesh, look, color). Stats roll from identity.
	pub fn random_clothing_item(&mut self) -> InventoryItem {
		let mesh = *self.choose(ClothingMesh::VALUES).unwrap_or(&ClothingMesh::TankTop);
		self.random_item_for_mesh(mesh)
	}

	fn random_item_for_mesh(&mut self, mesh: ClothingMesh) -> InventoryItem {
		let material = *self.choose(ClothingMaterial::VALUES).unwrap_or(&ClothingMaterial::Cloth);
		let color = *self.choose(ItemColor::VALUES).unwrap_or(&ItemColor::Natural);
		InventoryItem::clothing(mesh, material, color)
	}

	/// Starter roll: one lower, one upper, then unique “any” fills to `count`.
	pub fn random_starter_clothing(&mut self, count: usize) -> Vec<InventoryItem> {
		let lower = *self.choose(ClothingKind::STARTER_LOWERS).unwrap_or(&ClothingMesh::Pants);
		let upper = *self.choose(ClothingKind::STARTER_UPPERS).unwrap_or(&ClothingMesh::TankTop);
		let mut items = vec![self.random_item_for_mesh(lower), self.random_item_for_mesh(upper)];
		let mut remaining: Vec<ClothingMesh> = ClothingMesh::VALUES
			.iter()
			.copied()
			.filter(|mesh| *mesh != lower && *mesh != upper)
			.collect();
		let extra = count.saturating_sub(items.len()).min(remaining.len());
		for _ in 0..extra {
			let index = self.gen_index(remaining.len());
			let mesh = remaining.swap_remove(index);
			items.push(self.random_item_for_mesh(mesh));
		}
		items
	}

	/// Unique firearms for the starter bag.
	pub fn random_starter_firearms(&mut self, count: usize) -> Vec<InventoryItem> {
		let mut remaining: Vec<FirearmMesh> = FirearmMesh::VALUES.to_vec();
		let mut items = Vec::new();
		let take = count.min(remaining.len());
		for _ in 0..take {
			let index = self.gen_index(remaining.len());
			let mesh = remaining.swap_remove(index);
			items.push(InventoryItem::from_firearm_spec(FirearmSpec::roll(self, mesh)));
		}
		items
	}

	/// One skill map: a unique kind and a rolled seed. Not every starter shares
	/// the same pair of maps.
	pub fn random_starter_skill_maps(&mut self, count: usize) -> Vec<InventoryItem> {
		let mut remaining: Vec<SkillMapKind> = SkillMapKind::VALUES.to_vec();
		let mut items = Vec::new();
		let take = count.min(remaining.len()).min(SKILL_MAP_BAG_LIMIT);
		for _ in 0..take {
			let index = self.gen_index(remaining.len());
			let kind = remaining.swap_remove(index);
			items.push(InventoryItem::skill_map(SkillMapSpec::new(
				kind,
				self.in_range(1, u32::MAX),
			)));
		}
		items
	}

	/// Clothing starter, two unique firearms, one grenade, and one rolled skill map.
	pub fn random_starter_loadout(&mut self) -> Vec<InventoryItem> {
		let mut items = self.random_starter_clothing(STARTER_CLOTHING_COUNT);
		items.extend(self.random_starter_firearms(STARTER_WEAPON_COUNT));
		items.push(InventoryItem::rolled_grenade(self));
		items.extend(self.random_starter_skill_maps(STARTER_SKILL_MAP_COUNT));
		items
	}

	/// Generated firearms for a visual gallery. Bodies may repeat after the catalog
	/// is exhausted so the grid can be larger than the body list.
	pub fn random_gallery_firearms(&mut self, count: usize) -> Vec<InventoryItem> {
		(0..count)
			.map(|_| {
				let body = *self.choose(FirearmMesh::VALUES).unwrap_or(&FirearmMesh::Bullpup);
				InventoryItem::from_firearm_spec(FirearmSpec::roll(self, body))
			})
			.collect()
	}
}

/// High-word tag so item-icon keys never collide with [`SkillMapSpec::catalog_key`].
pub const ITEM_ICON_CATALOG_TAG: u64 = 0xC0DE_0000_0000_0000;

/// FNV-1a catalog key for a capture-id label.
pub fn icon_catalog_key_for_label(label: &str) -> u64 {
	ITEM_ICON_CATALOG_TAG | (Fnv1a64::digest(label.as_bytes()) & 0x0000_FFFF_FFFF_FFFF)
}

/// Catalog thumbnail key for the Self Lubikan tile.
pub fn self_lubikan_catalog_key() -> u64 {
	icon_catalog_key_for_label(SELF_LUBIKAN_CAPTURE_LABEL)
}

fn remap_selected(selected: &[usize], remap: &[Option<usize>]) -> Vec<usize> {
	selected
		.iter()
		.filter_map(|&index| remap.get(index).copied().flatten())
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{ClothingKind, ClothingMaterial, ClothingMesh, ItemColor};

	#[test]
	fn starter_rolls_are_unique_meshes() {
		let mut rng = ItemRng::from_seed(7);
		let items = rng.random_starter_clothing(STARTER_CLOTHING_COUNT);
		assert_eq!(items.len(), STARTER_CLOTHING_COUNT);
		let mut meshes: Vec<_> = items.iter().filter_map(InventoryItem::mesh).collect();
		meshes.sort_by_key(|mesh| mesh.label());
		meshes.dedup();
		assert_eq!(meshes.len(), STARTER_CLOTHING_COUNT);
		assert!(items
			.iter()
			.all(|item| item.clothing_stats().is_some_and(|stats| stats.weight > 0)));
		assert_eq!(items[0].mesh().map(ClothingMesh::kind), Some(ClothingKind::Lower));
		assert_eq!(items[1].mesh().map(ClothingMesh::kind), Some(ClothingKind::Upper));
		assert!(ClothingKind::STARTER_LOWERS.contains(&items[0].mesh().unwrap()));
		assert!(ClothingKind::STARTER_UPPERS.contains(&items[1].mesh().unwrap()));
	}

	#[test]
	fn starter_outfit_wears_lower_and_upper_and_queues_guns() {
		let items = vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			),
			InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Cloth, ItemColor::Red),
			InventoryItem::clothing(ClothingMesh::Robe, ClothingMaterial::Cloth, ItemColor::Cool),
			InventoryItem::firearm(FirearmMesh::Bullpup),
			InventoryItem::firearm(FirearmMesh::Reltor),
		];
		let inventory = Inventory::with_starter_outfit(items);
		assert_eq!(inventory.clothing, vec![0, 1]);
		assert_eq!(inventory.weapons, vec![3, 4]);
		assert!(!inventory.is_worn(2));
		assert_eq!(inventory.rank(3), Some(1));
		assert_eq!(inventory.rank(4), Some(2));
		assert_eq!(
			inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
	}

	#[test]
	fn wear_cap_rejects_a_seventh_item() {
		let items: Vec<_> = ClothingMesh::VALUES
			.iter()
			.take(WORN_CLOTHING_LIMIT + 1)
			.map(|mesh| InventoryItem::clothing(*mesh, ClothingMaterial::Cloth, ItemColor::Natural))
			.collect();
		let mut inventory = Inventory::with_all_worn(items);
		assert_eq!(inventory.clothing.len(), WORN_CLOTHING_LIMIT);
		assert!(!inventory.toggle_worn(WORN_CLOTHING_LIMIT));
		assert!(inventory.toggle_worn(0));
		assert_eq!(inventory.clothing.len(), WORN_CLOTHING_LIMIT - 1);
		assert!(inventory.toggle_worn(WORN_CLOTHING_LIMIT));
	}

	#[test]
	fn swap_active_rotates_the_weapon_queue() {
		let items = vec![
			InventoryItem::firearm(FirearmMesh::Bullpup),
			InventoryItem::firearm(FirearmMesh::Reltor),
			InventoryItem::firearm(FirearmMesh::Snailer),
		];
		let mut inventory =
			Inventory { items, clothing: Vec::new(), weapons: vec![0, 1, 2], skills: Vec::new() };
		assert!(inventory.swap_active());
		assert_eq!(inventory.weapons, vec![1, 2, 0]);
		assert_eq!(
			inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		assert!(inventory.swap_active());
		assert_eq!(inventory.weapons, vec![2, 0, 1]);
		let mut one = Inventory { weapons: vec![0], ..inventory.clone() };
		assert!(!one.swap_active());
		assert!(!Inventory::default().swap_active());
	}

	#[test]
	fn weapon_queue_caps_at_three_and_compacts_rank() {
		let items: Vec<_> =
			FirearmMesh::VALUES.iter().map(|mesh| InventoryItem::firearm(*mesh)).collect();
		let mut inventory =
			Inventory { items, clothing: Vec::new(), weapons: vec![0, 1, 2], skills: Vec::new() };
		assert!(!inventory.toggle(3));
		assert!(inventory.toggle(1));
		assert_eq!(inventory.weapons, vec![0, 2]);
		assert_eq!(inventory.rank(0), Some(1));
		assert_eq!(inventory.rank(2), Some(2));
		assert!(inventory.toggle(3));
		assert_eq!(inventory.weapons, vec![0, 2, 3]);
	}

	#[test]
	fn starter_loadout_has_clothes_two_guns_a_grenade_and_one_skill_map() {
		let mut rng = ItemRng::from_seed(42);
		let items = rng.random_starter_loadout();
		assert_eq!(
			items.len(),
			STARTER_CLOTHING_COUNT
				+ STARTER_WEAPON_COUNT
				+ STARTER_GRENADE_COUNT
				+ STARTER_SKILL_MAP_COUNT
		);
		assert_eq!(items.iter().filter(|item| item.slot() == InventorySlot::Clothing).count(), 3);
		assert_eq!(items.iter().filter(|item| item.slot() == InventorySlot::Weapons).count(), 3);
		assert_eq!(items.iter().filter_map(InventoryItem::grenade_spec).count(), 1);
		let blast = items.iter().find_map(InventoryItem::grenade_stats).expect("grenade");
		assert!((crate::BLAST_RADIUS_MIN..=crate::BLAST_RADIUS_MAX).contains(&blast.blast_radius));
		assert!((crate::LAUNCH_SPEED_MIN..=crate::LAUNCH_SPEED_MAX).contains(&blast.launch_speed));
		assert_eq!(items.iter().filter(|item| item.slot() == InventorySlot::Skills).count(), 1);
		let mut guns: Vec<_> = items.iter().filter_map(InventoryItem::firearm_mesh).collect();
		guns.sort_by_key(|mesh| mesh.label());
		guns.dedup();
		assert_eq!(guns.len(), 2);
		let outfit = Inventory::with_starter_outfit(items);
		assert_eq!(outfit.weapons.len(), 3);
		assert_eq!(outfit.skills.len(), 1);
		assert!(outfit.primary_skill_map().and_then(InventoryItem::skill_map_spec).is_some());
	}

	#[test]
	fn starter_skill_map_kind_depends_on_the_seed() {
		let mut a_rng = ItemRng::from_seed(1);
		let mut b_rng = ItemRng::from_seed(2);
		let a = a_rng.random_starter_skill_maps(STARTER_SKILL_MAP_COUNT);
		let b = b_rng.random_starter_skill_maps(STARTER_SKILL_MAP_COUNT);
		assert_eq!(a.len(), 1);
		assert_eq!(b.len(), 1);
		assert_ne!(a[0].skill_map_spec(), b[0].skill_map_spec());
		let kinds: std::collections::HashSet<_> = [1_u64, 2, 3, 4, 5, 6, 7, 8]
			.into_iter()
			.filter_map(|seed| {
				let mut rng = ItemRng::from_seed(seed);
				rng.random_starter_skill_maps(1)
					.first()
					.and_then(InventoryItem::skill_map_spec)
					.map(|spec| spec.kind)
			})
			.collect();
		assert!(kinds.len() > 1, "starter kind should not be the same for every seed");
	}

	#[test]
	fn cycle_skills_rotates_the_equipped_queue() {
		let items = vec![
			InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
			InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
		];
		let mut inventory =
			Inventory { items, clothing: Vec::new(), weapons: Vec::new(), skills: vec![0, 1] };
		assert!(inventory.cycle_skills(1));
		assert_eq!(inventory.skills, vec![1, 0]);
		assert_eq!(
			inventory.primary_skill_map().and_then(InventoryItem::skill_map_spec),
			Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))
		);
		assert!(inventory.cycle_skills(-1));
		assert_eq!(inventory.skills, vec![0, 1]);
		assert!(!Inventory::default().cycle_skills(1));
	}

	#[test]
	fn skill_queue_caps_at_three() {
		let items: Vec<_> = (0..4)
			.map(|seed| InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, seed)))
			.collect();
		let mut inventory =
			Inventory { items, clothing: Vec::new(), weapons: Vec::new(), skills: vec![0, 1, 2] };
		assert!(!inventory.toggle(3));
		assert!(inventory.toggle(1));
		assert_eq!(inventory.skills, vec![0, 2]);
		assert!(inventory.toggle(3));
		assert_eq!(inventory.skills, vec![0, 2, 3]);
	}

	#[test]
	fn seeded_rng_is_deterministic() {
		let mut a_rng = ItemRng::from_seed(42);
		let mut b_rng = ItemRng::from_seed(42);
		let a = a_rng.random_starter_loadout();
		let b = b_rng.random_starter_loadout();
		assert_eq!(a, b);
		assert_ne!(a[0].name(), a[0].label());
	}

	#[test]
	fn gallery_can_exceed_the_body_catalog() {
		let mut rng = ItemRng::from_seed(1);
		let items = rng.random_gallery_firearms(20);
		assert_eq!(items.len(), 20);
		assert!(items.iter().all(|item| item.firearm_spec().is_some()));
	}

	#[test]
	fn take_all_drains_items_and_clears_source_selections() {
		let mut bag = Inventory {
			items: vec![
				InventoryItem::clothing(
					ClothingMesh::Pants,
					ClothingMaterial::Cloth,
					ItemColor::Natural,
				),
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::clothing(
					ClothingMesh::Robe,
					ClothingMaterial::Cloth,
					ItemColor::Cool,
				),
			],
			clothing: vec![0],
			weapons: vec![1],
			skills: Vec::new(),
		};

		let taken = bag.take_all();

		assert!(bag.items.is_empty());
		assert!(bag.clothing.is_empty());
		assert!(bag.weapons.is_empty());
		assert!(bag.skills.is_empty());
		assert_eq!(taken.items.len(), 3);
		assert_eq!(taken.clothing, vec![0]);
		assert_eq!(taken.weapons, vec![1]);
		assert_eq!(
			taken.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
	}

	#[test]
	fn absorb_appends_without_auto_wear_or_remap() {
		let mut dest = Inventory {
			items: vec![InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			)],
			clothing: vec![0],
			weapons: Vec::new(),
			skills: Vec::new(),
		};
		let incoming = Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Reltor),
				InventoryItem::clothing(
					ClothingMesh::TankTop,
					ClothingMaterial::Cloth,
					ItemColor::Red,
				),
			],
			clothing: vec![1],
			weapons: vec![0],
			skills: Vec::new(),
		};

		dest.absorb(incoming);

		assert_eq!(dest.items.len(), 3);
		assert_eq!(dest.clothing, vec![0]);
		assert!(dest.weapons.is_empty());
		assert_eq!(dest.items[0].mesh(), Some(ClothingMesh::Pants));
		assert_eq!(dest.items[1].firearm_mesh(), Some(FirearmMesh::Reltor));
		assert_eq!(dest.items[2].mesh(), Some(ClothingMesh::TankTop));
		assert!(!dest.is_worn(2));
		assert_eq!(dest.rank(1), None);
	}

	#[test]
	fn explode_makes_one_visualized_bag_per_item() {
		let bag = Inventory {
			items: vec![
				InventoryItem::clothing(
					ClothingMesh::Pants,
					ClothingMaterial::Cloth,
					ItemColor::Natural,
				),
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::clothing(
					ClothingMesh::Robe,
					ClothingMaterial::Cloth,
					ItemColor::Cool,
				),
			],
			clothing: vec![0],
			weapons: vec![1],
			skills: Vec::new(),
		};

		let exploded = bag.explode();

		assert_eq!(exploded.len(), 3);
		assert!(exploded.iter().all(|piece| piece.items.len() == 1));
		assert_eq!(exploded[0].clothing, vec![0]);
		assert!(exploded[0].weapons.is_empty());
		assert_eq!(exploded[1].weapons, vec![0]);
		assert!(exploded[1].clothing.is_empty());
		assert_eq!(exploded[2].clothing, vec![0]);
		assert_eq!(exploded[0].items[0].mesh(), Some(ClothingMesh::Pants));
		assert_eq!(exploded[1].items[0].firearm_mesh(), Some(FirearmMesh::Bullpup));
		assert_eq!(exploded[2].items[0].mesh(), Some(ClothingMesh::Robe));
	}

	#[test]
	fn explode_empty_is_empty() {
		assert!(Inventory::default().explode().is_empty());
	}

	fn mixed_three() -> Inventory {
		Inventory {
			items: vec![
				InventoryItem::clothing(
					ClothingMesh::Pants,
					ClothingMaterial::Cloth,
					ItemColor::Natural,
				),
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::clothing(
					ClothingMesh::Robe,
					ClothingMaterial::Cloth,
					ItemColor::Cool,
				),
			],
			clothing: vec![0],
			weapons: vec![1],
			skills: Vec::new(),
		}
	}

	#[test]
	fn one_third_of_three_items_is_exactly_one() {
		let mut bag = mixed_three();
		let taken = bag.take_fraction(&mut ItemRng::from_seed(7), LootFraction::ONE_THIRD);
		assert_eq!(taken.items.len(), 1);
		assert_eq!(bag.items.len(), 2);
		assert_eq!(taken.items.len() + bag.items.len(), 3);
	}

	#[test]
	fn one_twelfth_of_twelve_items_is_exactly_one() {
		let items: Vec<_> = (0..12)
			.map(|index| {
				if index % 2 == 0 {
					InventoryItem::firearm(FirearmMesh::Bullpup)
				} else {
					InventoryItem::clothing(
						ClothingMesh::Pants,
						ClothingMaterial::Cloth,
						ItemColor::Natural,
					)
				}
			})
			.collect();
		let mut bag = Inventory::with_all_worn(items);
		let taken = bag.take_fraction(&mut ItemRng::from_seed(3), LootFraction::ONE_TWELFTH);
		assert_eq!(taken.items.len(), 1);
		assert_eq!(bag.items.len(), 11);
	}

	#[test]
	fn take_fraction_is_seeded_and_remaps_selections() {
		let selected = Inventory { clothing: vec![0, 2], weapons: vec![1], ..mixed_three() };
		let mut first = selected.clone();
		let mut second = selected;
		let taken_a = first.take_fraction(&mut ItemRng::from_seed(99), LootFraction::ONE_THIRD);
		let taken_b = second.take_fraction(&mut ItemRng::from_seed(99), LootFraction::ONE_THIRD);
		assert_eq!(taken_a, taken_b);
		assert_eq!(first, second);
		assert_eq!(taken_a.items.len(), 1);
		let kept = &taken_a.items[0];
		match kept.slot() {
			InventorySlot::Clothing => assert_eq!(taken_a.clothing, vec![0]),
			InventorySlot::Weapons => assert_eq!(taken_a.weapons, vec![0]),
			InventorySlot::Skills => assert_eq!(taken_a.skills, vec![0]),
		}
		assert!(first.items.iter().all(|item| item != kept));
	}

	#[test]
	fn none_takes_nothing_and_all_drains_the_bag() {
		let mut none = mixed_three();
		let taken = none.take_fraction(&mut ItemRng::from_seed(1), LootFraction::NONE);
		assert!(taken.items.is_empty());
		assert_eq!(none.items.len(), 3);
		let mut all = mixed_three();
		let taken = all.take_fraction(&mut ItemRng::from_seed(1), LootFraction::ALL);
		assert_eq!(taken.items.len(), 3);
		assert!(all.items.is_empty());
	}

	#[test]
	fn capture_ids_are_stable_and_distinct() -> anyhow::Result<()> {
		let bullpup = InventoryItem::firearm(FirearmMesh::Bullpup);
		let reltor = InventoryItem::firearm(FirearmMesh::Reltor);
		let pants = InventoryItem::clothing(
			ClothingMesh::Pants,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let painted =
			InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red);
		let a = bullpup.capture_id_label().ok_or_else(|| anyhow::anyhow!("bullpup"))?;
		let b = reltor.capture_id_label().ok_or_else(|| anyhow::anyhow!("reltor"))?;
		assert_ne!(a, b);
		assert_eq!(a, bullpup.capture_id_label().ok_or_else(|| anyhow::anyhow!("stable"))?);
		assert_ne!(
			pants.capture_id_label().ok_or_else(|| anyhow::anyhow!("pants"))?,
			painted.capture_id_label().ok_or_else(|| anyhow::anyhow!("painted"))?
		);
		assert_eq!(
			InventoryItem::standard_grenade().capture_id_label().as_deref(),
			Some("grenade:Grenade")
		);
		assert_eq!(
			InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1))
				.capture_id_label()
				.as_deref(),
			Some("skill-map:fireball:1")
		);
		Ok(())
	}

	#[test]
	fn icon_catalog_keys_stay_out_of_the_skill_map_namespace() -> anyhow::Result<()> {
		let item = InventoryItem::firearm(FirearmMesh::Bullpup);
		let key = item.icon_catalog_key().ok_or_else(|| anyhow::anyhow!("firearm key"))?;
		assert_eq!(key & ITEM_ICON_CATALOG_TAG, ITEM_ICON_CATALOG_TAG);
		let map = SkillMapSpec::new(SkillMapKind::Fireball, u32::MAX).catalog_key();
		assert_ne!(key & ITEM_ICON_CATALOG_TAG, map & ITEM_ICON_CATALOG_TAG);
		assert_eq!(
			icon_catalog_key_for_label(
				&item.capture_id_label().ok_or_else(|| anyhow::anyhow!("label"))?
			),
			key
		);
		assert_eq!(
			self_lubikan_catalog_key(),
			icon_catalog_key_for_label(SELF_LUBIKAN_CAPTURE_LABEL)
		);
		assert_ne!(self_lubikan_catalog_key(), key);
		Ok(())
	}

	#[test]
	fn remainder_roll_stays_near_the_expected_rate() {
		let drops: usize = (0..240)
			.map(|seed| ItemRng::from_seed(seed).sample_fraction_count(3, 1, 12))
			.sum();
		assert!(
			(40..=80).contains(&drops),
			"3 × 1/12 over 240 seeds should stay near 60, got {drops}"
		);
	}
}
