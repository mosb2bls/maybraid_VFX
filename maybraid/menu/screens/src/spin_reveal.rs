//! Spin-and-reveal screen for starter clothing and weapons.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, LineBreak};
use character_items::{InventoryItem, InventorySlot};
use maybraid_menu_controller::MenuController;
use menu_components::{
	republish_menu_activate, screen_back_scene, ButtonWithSubtext, SpinningIcon, TextMenuPlugin,
	BARLOW_SEMIBOLD, LOADING_ICON_SIZE, MENU_CLEAR, PANEL_BLOCK_FONT_SIZE, PANEL_LABEL_FONT_SIZE,
	SPIN_REVEAL_SECS, SPIN_REVEAL_TILE_HEIGHT, SPIN_REVEAL_TILE_WIDTH, TEXT_YELLOW,
	TEXT_YELLOW_FAINT,
};

use crate::input::add_menu_input;
use crate::show::take_menu_show_request;
use crate::MenuScreen;

/// Queue a spin-and-reveal spawn. Pair with [`SpinRevealItems`].
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowSpinReveal;

/// Items to reveal, in order.
#[derive(Resource, Clone, Debug)]
pub struct SpinRevealItems(pub Vec<InventoryItem>);

/// The garment currently on the roll, for hosts that spawn a live preview.
#[derive(Resource, Clone, Debug)]
pub struct SpinRevealCurrent {
	pub item: InventoryItem,
	pub revealed: bool,
}

/// Marker on the spawned spin-and-reveal root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SpinRevealScreen;

/// Advance the current card, or finish after the last reveal.
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum SpinRevealChoice {
	#[default]
	Advance,
}

/// Fired once every item has been revealed and the player continues.
#[derive(Message, Clone, Debug)]
pub struct SpinRevealFinished {
	pub items: Vec<InventoryItem>,
}

#[derive(Resource, Debug, Clone)]
struct SpinRevealPhase {
	index: usize,
	spinning: bool,
	elapsed: f32,
	dirty: bool,
}

impl SpinRevealPhase {
	fn start() -> Self {
		Self { index: 0, spinning: true, elapsed: 0.0, dirty: true }
	}
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
				(apply_show_spin_reveal, tick_spin_reveal, finish_spin_reveal, rebuild_spin_reveal)
					.chain()
					.in_set(SpinRevealSystems::Apply),
			);
	}
}

fn apply_show_spin_reveal(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowSpinReveal>>,
	existing: Query<Entity, With<MenuScreen>>,
	items: Option<Res<SpinRevealItems>>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
		return;
	}
	if items.is_none() {
		return;
	}
	commands.insert_resource(SpinRevealPhase::start());
}

fn tick_spin_reveal(time: Res<Time>, mut phase: Option<ResMut<SpinRevealPhase>>) {
	let Some(phase) = phase.as_mut() else {
		return;
	};
	if !phase.spinning {
		return;
	}
	phase.elapsed += time.delta_secs();
	if phase.elapsed >= SPIN_REVEAL_SECS {
		phase.spinning = false;
		phase.dirty = true;
	}
}

fn rebuild_spin_reveal(
	mut commands: Commands,
	items: Option<Res<SpinRevealItems>>,
	mut phase: Option<ResMut<SpinRevealPhase>>,
	screens: Query<Entity, With<SpinRevealScreen>>,
) {
	let Some(phase) = phase.as_mut() else {
		return;
	};
	if !phase.dirty {
		return;
	}
	let Some(items) = items else {
		return;
	};
	for entity in &screens {
		commands.entity(entity).despawn();
	}
	if items.0.is_empty() {
		phase.dirty = false;
		return;
	}
	let index = phase.index.min(items.0.len().saturating_sub(1));
	let revealed = !phase.spinning;
	let total = items.0.len();
	let action = action_copy(&items.0[index], revealed, index + 1 >= total, index, total);
	commands.insert_resource(SpinRevealCurrent { item: items.0[index].clone(), revealed });
	commands.spawn_scene(spin_scene(&items.0[index], revealed, action));
	phase.dirty = false;
}

fn action_copy(
	item: &InventoryItem,
	revealed: bool,
	last: bool,
	index: usize,
	total: usize,
) -> (String, String) {
	if revealed && last {
		("Continue".into(), "Edit Character".into())
	} else {
		let kind = match item.slot() {
			InventorySlot::Clothing => "Clothing Item",
			InventorySlot::Weapons => "Weapon",
			InventorySlot::Skills => "Skill Map",
		};
		("Next".into(), format!("{kind} ({}/{})", index + 1, total))
	}
}

fn spin_scene(
	item: &InventoryItem,
	revealed: bool,
	action: (String, String),
) -> impl Scene + 'static {
	let name = item.name();
	let (label, subtext) = action;
	let mut children: Vec<Box<dyn Scene>> = vec![
		Box::new(ButtonWithSubtext::new(label, subtext, SpinRevealChoice::Advance).scene()),
		Box::new(screen_back_scene()),
	];
	if revealed {
		children.push(Box::new(name_below_preview(name, item.catalog_detail())));
	} else {
		children.push(Box::new(centered_spinner()));
	}
	let background = if revealed { Color::NONE } else { MENU_CLEAR };
	bsn! {
		SpinRevealScreen
		MenuScreen
		MenuController
		BackgroundColor(background)
		Node {
			width: percent(100),
			height: percent(100),
		}
		Pickable::IGNORE
		on(republish_menu_activate::<SpinRevealChoice>)
		Children [ {children} ]
	}
}

fn centered_spinner() -> impl Scene + 'static {
	let mark: Vec<Box<dyn Scene>> =
		vec![Box::new(SpinningIcon::maybraid_scene(LOADING_ICON_SIZE, TEXT_YELLOW))];
	bsn! {
		Node {
			position_type: PositionType::Absolute,
			width: percent(100),
			height: percent(100),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
		}
		Pickable::IGNORE
		Children [ {mark} ]
	}
}

fn name_below_preview(name: String, detail: String) -> impl Scene + 'static {
	let mut caption: Vec<Box<dyn Scene>> =
		vec![Box::new(caption_line(name, PANEL_BLOCK_FONT_SIZE, TEXT_YELLOW))];
	if !detail.is_empty() {
		caption.push(Box::new(caption_line(detail, PANEL_LABEL_FONT_SIZE, TEXT_YELLOW_FAINT)));
	}
	let margin = UiRect { top: Val::Px(SPIN_REVEAL_TILE_HEIGHT / 2.0 + 16.0), ..default() };
	bsn! {
		Node {
			position_type: PositionType::Absolute,
			top: percent(50),
			width: percent(100),
			margin: margin,
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Center,
		}
		Pickable::IGNORE
		Children [ {caption} ]
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
			width: px(SPIN_REVEAL_TILE_WIDTH),
		}
		Pickable::IGNORE
	}
}

fn finish_spin_reveal(
	mut commands: Commands,
	mut choices: MessageReader<SpinRevealChoice>,
	mut finished: MessageWriter<SpinRevealFinished>,
	items: Option<Res<SpinRevealItems>>,
	mut phase: Option<ResMut<SpinRevealPhase>>,
) {
	if !choices.read().any(|choice| *choice == SpinRevealChoice::Advance) {
		return;
	}
	let Some(phase) = phase.as_mut() else {
		return;
	};
	let Some(items) = items else {
		return;
	};
	if phase.spinning {
		phase.spinning = false;
		phase.dirty = true;
		return;
	}
	if phase.index + 1 < items.0.len() {
		phase.index += 1;
		phase.spinning = true;
		phase.elapsed = 0.0;
		phase.dirty = true;
		return;
	}
	finished.write(SpinRevealFinished { items: items.0.clone() });
	commands.remove_resource::<SpinRevealPhase>();
	commands.remove_resource::<SpinRevealCurrent>();
}

impl SpinRevealScreen {
	pub fn scene(item: &InventoryItem, revealed: bool) -> impl Scene + 'static {
		spin_scene(item, revealed, action_copy(item, revealed, true, 0, 1))
	}
}
