//! Frosted corner-plate chrome shared by combat HUD panels.

use bevy::prelude::*;

pub(crate) const HUD_CORNER_INSET: f32 = 16.0;

const PLATE_PAD_X: f32 = 8.0;
const PLATE_PAD_Y: f32 = 6.0;
const PLATE_BORDER: f32 = 1.5;
const PLATE_RADIUS: f32 = 8.0;
const PLATE_FILL: Color = Color::srgba(0.52, 0.52, 0.56, 0.82);
const PLATE_STROKE: Color = Color::srgba(0.78, 0.78, 0.82, 0.9);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HudPlateCorner {
	BottomLeft,
	TopRight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HudPlateLayout {
	pub width: f32,
	pub row_gap: f32,
	pub corner: HudPlateCorner,
}

impl HudPlateLayout {
	fn node(self) -> Node {
		let mut node = Node {
			position_type: PositionType::Absolute,
			width: Val::Px(self.width),
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Stretch,
			row_gap: Val::Px(self.row_gap),
			padding: UiRect::axes(Val::Px(PLATE_PAD_X), Val::Px(PLATE_PAD_Y)),
			border: UiRect::all(Val::Px(PLATE_BORDER)),
			border_radius: BorderRadius::all(Val::Px(PLATE_RADIUS)),
			..default()
		};
		match self.corner {
			HudPlateCorner::BottomLeft => {
				node.left = Val::Px(HUD_CORNER_INSET);
				node.bottom = Val::Px(HUD_CORNER_INSET);
			}
			HudPlateCorner::TopRight => {
				node.right = Val::Px(HUD_CORNER_INSET);
				node.top = Val::Px(HUD_CORNER_INSET);
			}
		}
		node
	}
}

/// Spawns a hidden, pick-ignored HUD plate with shared fill, stroke, and padding.
pub(crate) fn spawn_hud_plate<M: Component>(
	parent: &mut ChildSpawnerCommands,
	name: &'static str,
	marker: M,
	layout: HudPlateLayout,
	children: impl FnOnce(&mut ChildSpawnerCommands),
) -> Entity {
	parent
		.spawn((
			Name::new(name),
			marker,
			layout.node(),
			BackgroundColor(PLATE_FILL),
			BorderColor::all(PLATE_STROKE),
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(children)
		.id()
}
