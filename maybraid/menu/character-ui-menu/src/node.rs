//! Renderer-agnostic menu intermediate representation.
//!
//! Species menus lower their typed state into a tree of well-known
//! [`MenuNode`]s. Renderers implement a narrow forward contract over these
//! variants and never see species-specific types. Values that used to be
//! resolved imperatively while painting (labels, option lists, preview tints)
//! are resolved here, at lowering time, so the tree is plain data.

use crate::{
	AssetOption, AssetSingleSelect, LabelOption, ListValues, SingleSelect, Slider, SwatchOption,
	SwatchSingleSelect, ThumbnailCamera,
};

/// sRGBA tint applied to asset thumbnails, resolved when a menu lowers to IR.
///
/// This replaces the old imperative `RenderContext.preview_color` scoping:
/// every node that previews assets carries its tint explicitly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewColor {
	pub red: f32,
	pub green: f32,
	pub blue: f32,
	pub alpha: f32,
}

impl PreviewColor {
	pub const WHITE: Self = Self { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 };

	pub const fn srgb(red: f32, green: f32, blue: f32) -> Self {
		Self { red, green, blue, alpha: 1.0 }
	}

	/// Resolves a palette value's preview tint from its swatch hex.
	pub fn of<T: SwatchOption>(value: T) -> Self {
		Self::from_hex(value.color_hex())
	}

	/// Parses a `#RRGGBB` hex string; malformed input falls back to white.
	pub fn from_hex(hex: &str) -> Self {
		let hex = hex.strip_prefix('#').unwrap_or(hex);
		if hex.len() != 6 {
			return Self::WHITE;
		}
		let channel = |range: core::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).ok();
		match (channel(0..2), channel(2..4), channel(4..6)) {
			(Some(red), Some(green), Some(blue)) => {
				Self::srgb(red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0)
			}
			_ => Self::WHITE,
		}
	}
}

/// One tile in a select row (species picker and similar).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectChoice<E> {
	pub label: &'static str,
	pub selected: bool,
	pub event: E,
}

/// Labeled subgroup of select tiles inside a [`MenuNode::SectionSelect`].
#[derive(Clone, Debug, PartialEq)]
pub struct SelectGroup<E> {
	/// Subheading above the choice row; omit for a single unlabeled row.
	pub label: Option<&'static str>,
	pub choices: Vec<SelectChoice<E>>,
}

impl<E> SelectGroup<E> {
	pub fn unlabeled(choices: Vec<SelectChoice<E>>) -> Self {
		Self { label: None, choices }
	}

	pub fn labeled(label: &'static str, choices: Vec<SelectChoice<E>>) -> Self {
		Self { label: Some(label), choices }
	}

	pub fn from_values<T>(
		group_label: Option<&'static str>,
		selected: T,
		mut event: impl FnMut(T) -> E,
		values: &[T],
	) -> Self
	where
		T: LabelOption + PartialEq + Copy,
	{
		Self {
			label: group_label,
			choices: values
				.iter()
				.map(|value| SelectChoice {
					label: value.label(),
					selected: *value == selected,
					event: event(*value),
				})
				.collect(),
		}
	}
}

/// One color swatch in a swatch row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwatchChoice<E> {
	pub label: &'static str,
	pub color_hex: &'static str,
	pub selected: bool,
	pub event: E,
}

impl<E> SwatchChoice<E> {
	/// Lowers every value of a [`SwatchOption`] enum into a swatch row.
	pub fn from_values<T>(active: T, mut event: impl FnMut(T) -> E) -> Vec<Self>
	where
		T: SwatchOption + ListValues + PartialEq + Copy,
	{
		T::values()
			.iter()
			.map(|value| Self {
				label: value.label(),
				color_hex: value.color_hex(),
				selected: *value == active,
				event: event(*value),
			})
			.collect()
	}
}

/// One asset button in an asset grid or item row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssetChoice<E> {
	pub label: &'static str,
	pub path: &'static str,
	pub thumbnail_camera: ThumbnailCamera,
	pub selected: bool,
	pub event: E,
}

impl<E> AssetChoice<E> {
	pub fn new<T>(value: T, selected: bool, event: E) -> Self
	where
		T: AssetOption + Copy,
	{
		let asset = value.asset();
		Self {
			label: asset.label,
			path: asset.path,
			thumbnail_camera: asset.thumbnail_camera,
			selected,
			event,
		}
	}
}

/// One toggleable item with its own preview tint, color swatches, and look tiles.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemRow<E> {
	pub asset: AssetChoice<E>,
	pub preview: PreviewColor,
	pub colors: Vec<SwatchChoice<E>>,
	pub materials: Vec<SelectChoice<E>>,
}

/// One cell in a [`MenuNode::GridCatalog`]. Selection is marked by the host
/// (Maybraid son for clothing, 1-based rank for weapons).
#[derive(Clone, Debug, PartialEq)]
pub struct GridCatalogChoice<E> {
	pub label: String,
	/// Compact stat line under the name (weight, DPC, fire mode).
	pub detail: String,
	pub path: &'static str,
	pub thumbnail_camera: ThumbnailCamera,
	pub preview: PreviewColor,
	/// Host-rendered image (skill-map catalog). Checked before mesh path.
	pub image_key: Option<u64>,
	pub selected: bool,
	/// 1-based slot rank when this item is queued (weapons). Clothing leaves
	/// this `None` and uses the Maybraid son for selection.
	pub rank: Option<u8>,
	pub event: E,
}

/// Plus / minus / read-only tone for a loadout or catalog value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatTone {
	#[default]
	Neutral,
	Plus,
	Minus,
	/// Derived formula, e.g. the pace equation.
	Formula,
}

impl StatTone {
	pub fn from_i16(value: i16) -> Self {
		if value > 0 {
			Self::Plus
		} else if value < 0 {
			Self::Minus
		} else {
			Self::Neutral
		}
	}

	pub fn from_text(value: &str) -> Self {
		let trimmed = value.trim();
		if trimmed.starts_with('+') {
			Self::Plus
		} else if trimmed.starts_with('-') {
			Self::Minus
		} else {
			Self::Neutral
		}
	}
}

/// One labeled number inside a [`StatCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatLine {
	pub label: String,
	pub value: String,
	pub tone: StatTone,
}

impl StatLine {
	pub fn unsigned(label: impl Into<String>, value: impl Into<String>) -> Self {
		Self { label: label.into(), value: value.into(), tone: StatTone::Neutral }
	}

	pub fn from_display(label: impl Into<String>, value: impl Into<String>) -> Self {
		let value = value.into();
		let tone = StatTone::from_text(&value);
		Self { label: label.into(), value, tone }
	}

	pub fn formula(label: impl Into<String>, value: impl Into<String>) -> Self {
		Self { label: label.into(), value: value.into(), tone: StatTone::Formula }
	}
}

/// Titled column in a [`MenuNode::StatGrid`] (Total / Base / Buffs / weapons).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatCard {
	pub title: String,
	pub rows: Vec<StatLine>,
}

/// Renderer-agnostic menu tree. Generic over the host event type `E`, which is
/// embedded in leaves at lowering time; renderers forward it verbatim.
#[derive(Clone, Debug, PartialEq)]
pub enum MenuNode<E> {
	/// Invisible grouping for composition; flattened by [`normalize`] before
	/// rendering. Fragments never paint as their own widget.
	Fragment(Vec<MenuNode<E>>),
	/// Collapsible labeled section. Open state is keyed by label via
	/// [`crate::SectionOpen`].
	Section { label: &'static str, children: Vec<MenuNode<E>> },
	/// Tile picker whose selection decides the subtree below it
	/// (e.g. the species picker). Choices may be split into labeled groups for
	/// readability; only the selected subtree is lowered into `children`.
	SectionSelect { label: &'static str, groups: Vec<SelectGroup<E>>, children: Vec<MenuNode<E>> },
	/// `<` value `>` control with an inline label.
	LabeledCycle { label: &'static str, value: &'static str, minus: E, plus: E },
	/// `-` value `+` stepped scalar control with an inline label.
	LabeledSlider { label: &'static str, value: f32, decrease: E, increase: E },
	/// Inline-labeled row of color swatches.
	LabeledSwatch { label: &'static str, choices: Vec<SwatchChoice<E>> },
	/// Block-labeled grid of asset buttons with thumbnail previews.
	BlockAsset { label: &'static str, preview: PreviewColor, choices: Vec<AssetChoice<E>> },
	/// Block-labeled multi-select of items, each row pairing a toggle button
	/// with color swatches and look tiles (e.g. clothing layers).
	ItemMultiSelect { label: &'static str, rows: Vec<ItemRow<E>> },
	/// Wrapping grid of owned items with a selection cap (`max_selected`).
	GridCatalog { label: &'static str, max_selected: usize, choices: Vec<GridCatalogChoice<E>> },
	/// One-line text field. Toggle and typed changes are renderer events;
	/// the IR only carries the current value.
	ShortText { label: &'static str, value: String, max_len: usize },
	/// Full-width action (e.g. Save Character). Fires `event` on activate.
	Action { label: &'static str, event: E },
	/// Read-only labeled value (compiled loadout rows, item stats).
	LabeledValue { label: String, value: String },
	/// Wrapping grid of titled stat columns (loadout Total / Base / Buffs,
	/// then queued weapons).
	StatGrid { cards: Vec<StatCard> },
}

impl<E> MenuNode<E> {
	/// Groups `children` for composition; invisible after [`normalize`].
	pub fn fragment(children: impl IntoIterator<Item = MenuNode<E>>) -> Self {
		Self::Fragment(children.into_iter().collect())
	}

	/// Collapsible labeled section; `child` may be a [`Fragment`].
	pub fn section(label: &'static str, child: MenuNode<E>) -> Self {
		Self::Section { label, children: normalize(vec![child]) }
	}

	/// Alias for [`Self::section`].
	pub fn submenu(label: &'static str, child: MenuNode<E>) -> Self {
		Self::section(label, child)
	}

	pub fn section_select<T>(
		label: &'static str,
		selected: T,
		mut event: impl FnMut(T) -> E,
		child: MenuNode<E>,
	) -> Self
	where
		T: ListValues + LabelOption + PartialEq + Copy,
	{
		Self::SectionSelect {
			label,
			groups: vec![SelectGroup::from_values(None, selected, &mut event, T::values())],
			children: normalize(vec![child]),
		}
	}

	/// Like [`Self::section_select`], but splits choices into labeled groups.
	pub fn section_select_grouped<T>(
		label: &'static str,
		selected: T,
		mut event: impl FnMut(T) -> E,
		groups: &[(&'static str, &[T])],
		child: MenuNode<E>,
	) -> Self
	where
		T: LabelOption + PartialEq + Copy,
	{
		Self::SectionSelect {
			label,
			groups: groups
				.iter()
				.map(|(group_label, values)| {
					SelectGroup::from_values(Some(*group_label), selected, &mut event, values)
				})
				.collect(),
			children: normalize(vec![child]),
		}
	}

	/// `event` receives the cycle delta (`-1` / `1`).
	pub fn cycle<T>(
		label: &'static str,
		select: &SingleSelect<T>,
		mut event: impl FnMut(i32) -> E,
	) -> Self
	where
		T: LabelOption + Copy,
	{
		Self::LabeledCycle { label, value: select.value.label(), minus: event(-1), plus: event(1) }
	}

	/// `event` receives the signed step delta (`-step` / `step`).
	pub fn slider(label: &'static str, slider: &Slider, mut event: impl FnMut(f32) -> E) -> Self {
		Self::LabeledSlider {
			label,
			value: slider.value,
			decrease: event(-slider.step),
			increase: event(slider.step),
		}
	}

	pub fn swatch<T>(
		label: &'static str,
		swatch: &SwatchSingleSelect<T>,
		event: impl FnMut(T) -> E,
	) -> Self
	where
		T: SwatchOption + ListValues + PartialEq + Copy,
	{
		Self::LabeledSwatch { label, choices: SwatchChoice::from_values(swatch.value, event) }
	}

	pub fn asset_grid<T>(
		label: &'static str,
		select: &AssetSingleSelect<T>,
		preview: PreviewColor,
		mut event: impl FnMut(T) -> E,
	) -> Self
	where
		T: AssetOption + ListValues + PartialEq + Copy,
	{
		Self::BlockAsset {
			label,
			preview,
			choices: T::values()
				.iter()
				.map(|value| AssetChoice::new(*value, *value == select.value, event(*value)))
				.collect(),
		}
	}

	/// Owned-item grid. `max_selected` is the wear/equip cap the host enforces.
	pub fn grid_catalog(
		label: &'static str,
		max_selected: usize,
		choices: impl IntoIterator<Item = GridCatalogChoice<E>>,
	) -> Self {
		Self::GridCatalog { label, max_selected, choices: choices.into_iter().collect() }
	}

	/// Short typed label; `max_len` is a hard cap on the stored value.
	pub fn short_text(label: &'static str, value: impl Into<String>, max_len: usize) -> Self {
		let mut value = value.into();
		if value.chars().count() > max_len {
			value = value.chars().take(max_len).collect();
		}
		Self::ShortText { label, value, max_len }
	}

	/// Full-width labeled action that fires `event` when activated.
	pub fn action(label: &'static str, event: E) -> Self {
		Self::Action { label, event }
	}

	/// Read-only name / value pair.
	pub fn labeled_value(label: impl Into<String>, value: impl Into<String>) -> Self {
		Self::LabeledValue { label: label.into(), value: value.into() }
	}

	/// Wrapping grid of titled stat columns.
	pub fn stat_grid(cards: impl IntoIterator<Item = StatCard>) -> Self {
		Self::StatGrid { cards: cards.into_iter().collect() }
	}
}

/// Recursively removes [`MenuNode::Fragment`] wrappers.
pub fn normalize<E>(nodes: Vec<MenuNode<E>>) -> Vec<MenuNode<E>> {
	nodes
		.into_iter()
		.flat_map(|node| match node {
			MenuNode::Fragment(children) => normalize(children),
			other => vec![other],
		})
		.collect()
}

/// Lowers typed menu state to a renderer-agnostic [`MenuNode`] tree.
pub trait MenuComponent<E> {
	fn menu_node(&self) -> MenuNode<E>;

	/// Root node flattened for renderers that consume a slice.
	fn menu_nodes(&self) -> Vec<MenuNode<E>> {
		normalize(vec![self.menu_node()])
	}
}
