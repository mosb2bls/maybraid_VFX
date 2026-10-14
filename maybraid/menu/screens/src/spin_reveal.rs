//! All-at-once gallery of granted starter inventory.

use bevy::prelude::*;
use bevy::scene::prelude::{Scene, bsn, template_value};
use bevy::text::{FontSourceTemplate, LineBreak};
use character_items::InventoryItem;
use maybraid_menu_controller::MenuController;
use menu_components::{
	BARLOW_SEMIBOLD, ButtonWithSubtext, MENU_CLEAR, PANEL_ITEM_FONT_SIZE, PANEL_LABEL_FONT_SIZE,
	TEXT_YELLOW, TEXT_YELLOW_FAINT, TextMenuPlugin, republish_menu_activate, screen_back_scene,
};

use crate::MenuScreen;
use crate::input::add_menu_input;
use crate::show::{MenuScreens, MenuShowRequest, add_menu_show_systems};

/// Queue a starter-inventory gallery spawn. Pair with [`SpinRevealItems`].
#[derive(Component, Debug, Clone, Copy)]
#[require(MenuShowRequest = MenuShowRequest::next())]
pub struct RequestShowSpinReveal;

/// Granted items shown together on the gallery.
#[derive(Resource, Clone, Debug)]
pub struct SpinRevealItems(pub Vec<InventoryItem>);

/// Hosts bind catalog art here. The gallery is the only view; do not spawn a
/// live inspect mesh or camera on this screen.
#[derive(Resource, Clone, Debug)]
pub struct SpinRevealCurrent {
	pub items: Vec<InventoryItem>,
	pub revealed: bool,
}

/// Marker on the spawned starter-inventory gallery root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SpinRevealScreen;

/// Continue from the gallery into the body editor.
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum SpinRevealChoice {
	#[default]
	Advance,
}

/// Fired once the player continues from the gallery.
#[derive(Message, Clone, Debug)]
pub struct SpinRevealFinished {
	pub items: Vec<InventoryItem>,
}

pub fn request_show_spin_reveal(commands: &mut Commands, items: Vec<InventoryItem>) {
	commands.insert_resource(SpinRevealItems(items));
	commands.spawn(RequestShowSpinReveal);
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpinRevealSystems {
	Apply,
}

pub struct SpinRevealScreenPlugin;

impl Plugin for SpinRevealScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.add_plugins(TextMenuPlugin::<SpinRevealChoice>::default())
			.add_message::<SpinRevealFinished>()
			.add_systems(
				Update,
				(finish_spin_reveal, drop_stale_gallery)
					.chain()
					.in_set(SpinRevealSystems::Apply),
			);
		add_menu_show_systems(app, apply_show_spin_reveal);
	}
}

fn apply_show_spin_reveal(
	mut commands: Commands,
	requests: Query<(), With<RequestShowSpinReveal>>,
	screens: MenuScreens,
	items: Option<Res<SpinRevealItems>>,
) {
	if screens.take(&mut commands, &requests).is_none() {
		return;
	}
	let Some(items) = items else {
		return;
	};
	spawn_grant_gallery(&mut commands, &items);
}

/// If another screen replaced the gallery, drop the current-item binding so
/// hosts do not keep painting catalog art over the new screen.
fn drop_stale_gallery(
	mut commands: Commands,
	current: Option<Res<SpinRevealCurrent>>,
	screens: Query<Entity, With<SpinRevealScreen>>,
) {
	if current.is_none() || !screens.is_empty() {
		return;
	}
	commands.remove_resource::<SpinRevealCurrent>();
}

fn spawn_grant_gallery(commands: &mut Commands, items: &SpinRevealItems) {
	commands.insert_resource(SpinRevealCurrent { items: items.0.clone(), revealed: true });
	commands.spawn_scene(gallery_scene(items));
}

fn gallery_scene(items: &SpinRevealItems) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(
			ButtonWithSubtext::new("Continue", "Edit Character", SpinRevealChoice::Advance).scene(),
		),
		Box::new(screen_back_scene()),
		Box::new(gallery_grid(items.0.clone())),
	];
	bsn! {
		SpinRevealScreen
		MenuScreen
		MenuController
		BackgroundColor(MENU_CLEAR)
		Node {
			width: percent(100),
			height: percent(100),
		}
		Pickable::IGNORE
		on(republish_menu_activate::<SpinRevealChoice>)
		Children [ {children} ]
	}
}

fn gallery_grid(items: Vec<InventoryItem>) -> impl Scene + 'static {
	let tiles: Vec<Box<dyn Scene>> = items
		.iter()
		.map(|item| Box::new(gallery_tile(item)) as Box<dyn Scene>)
		.collect();
	bsn! {
		Node {
			position_type: PositionType::Absolute,
			top: percent(16),
			width: percent(100),
			height: percent(72),
			padding: UiRect::axes(Val::Px(40.0), Val::Px(12.0)),
			flex_direction: FlexDirection::Row,
			flex_wrap: FlexWrap::Wrap,
			justify_content: JustifyContent::Center,
			align_content: AlignContent::FlexStart,
			column_gap: Val::Px(16.0),
			row_gap: Val::Px(16.0),
		}
		Pickable::IGNORE
		Children [ {tiles} ]
	}
}

fn gallery_tile(item: &InventoryItem) -> impl Scene + 'static {
	let name = item.name();
	let detail = item.catalog_detail();
	let mut lines: Vec<Box<dyn Scene>> =
		vec![Box::new(caption_line(name, PANEL_ITEM_FONT_SIZE, TEXT_YELLOW))];
	if !detail.is_empty() {
		lines.push(Box::new(caption_line(detail, PANEL_LABEL_FONT_SIZE, TEXT_YELLOW_FAINT)));
	}
	bsn! {
		Node {
			width: px(210.0),
			min_height: px(88.0),
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Center,
			justify_content: JustifyContent::Center,
			padding: UiRect::all(Val::Px(8.0)),
		}
		Pickable::IGNORE
		Children [ {lines} ]
	}
}

fn caption_line(text: String, size: f32, color: Color) -> impl Scene + 'static {
	bsn! {
		template_value(Text::new(text))
		TextFont {
			font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
			font_size: px(size),
		}
		TextColor(color)
		TextLayout::new(Justify::Center, LineBreak::WordBoundary)
		Node {
			width: px(200.0),
		}
		Pickable::IGNORE
	}
}

fn finish_spin_reveal(
	mut commands: Commands,
	mut choices: MessageReader<SpinRevealChoice>,
	mut finished: MessageWriter<SpinRevealFinished>,
	items: Option<Res<SpinRevealItems>>,
) {
	if !choices.read().any(|choice| *choice == SpinRevealChoice::Advance) {
		return;
	}
	let Some(items) = items else {
		return;
	};
	finished.write(SpinRevealFinished { items: items.0.clone() });
	commands.remove_resource::<SpinRevealCurrent>();
}

impl SpinRevealScreen {
	pub fn scene(items: &[InventoryItem]) -> impl Scene + 'static {
		gallery_scene(&SpinRevealItems(items.to_vec()))
	}
}

#[cfg(test)]
mod tests {
	use super::{
		SpinRevealChoice, SpinRevealCurrent, SpinRevealFinished, SpinRevealItems, SpinRevealScreen,
		drop_stale_gallery, finish_spin_reveal,
	};
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use character_items::{ClothingMaterial, ClothingMesh, InventoryItem, ItemColor};

	fn pants() -> InventoryItem {
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red)
	}

	#[test]
	fn a_replaced_gallery_drops_the_current_binding() -> Result<(), Box<dyn std::error::Error>> {
		let mut world = World::new();
		world.insert_resource(SpinRevealCurrent { items: vec![pants()], revealed: true });
		world
			.run_system_once(drop_stale_gallery)
			.map_err(|error| format!("{error:?}"))?;
		world.flush();
		if world.get_resource::<SpinRevealCurrent>().is_some() {
			return Err("the current binding should end once the gallery is gone".into());
		}
		Ok(())
	}

	#[test]
	fn a_live_gallery_keeps_the_current_binding() -> Result<(), Box<dyn std::error::Error>> {
		let mut world = World::new();
		world.spawn(SpinRevealScreen);
		world.insert_resource(SpinRevealCurrent { items: vec![pants()], revealed: true });
		world
			.run_system_once(drop_stale_gallery)
			.map_err(|error| format!("{error:?}"))?;
		world.flush();
		if world.get_resource::<SpinRevealCurrent>().is_none() {
			return Err("a live gallery must keep its current binding".into());
		}
		Ok(())
	}

	#[test]
	fn continue_finishes_every_granted_item() -> Result<(), Box<dyn std::error::Error>> {
		let items = vec![
			pants(),
			InventoryItem::clothing(
				ClothingMesh::TankTop,
				ClothingMaterial::Hawaiian,
				ItemColor::Blue,
			),
		];
		let mut world = World::new();
		world.init_resource::<Messages<SpinRevealChoice>>();
		world.init_resource::<Messages<SpinRevealFinished>>();
		world.insert_resource(SpinRevealItems(items.clone()));
		world
			.run_system_once(|mut writer: MessageWriter<SpinRevealChoice>| {
				writer.write(SpinRevealChoice::Advance);
			})
			.map_err(|error| format!("{error:?}"))?;
		world
			.run_system_once(finish_spin_reveal)
			.map_err(|error| format!("{error:?}"))?;
		world.flush();
		let got = world
			.run_system_once(|mut reader: MessageReader<SpinRevealFinished>| {
				reader.read().next().map(|event| event.items.clone())
			})
			.map_err(|error| format!("{error:?}"))?;
		if got.as_ref() != Some(&items) {
			return Err("finished items must be the whole grant".into());
		}
		Ok(())
	}
}
