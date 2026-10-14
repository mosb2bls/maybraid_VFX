//! Centered loading page: spinning mark, bar, and explainer.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use maybraid_menu_controller::MenuController;
use menu_components::{
	set_loading_explainer, set_loading_progress, LoadingBarFill, LoadingExplainer, LoadingStack,
	MenuComponentsPlugin,
};

use crate::show::{spawn_menu_show_request, take_menu_show_request};
use crate::MenuScreen;

/// Queue a loading-screen spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowLoading;

/// Incoming fill, 0..=1. Applied to the visible [`LoadingScreen`].
#[derive(Message, Debug, Clone, Copy)]
pub struct LoadingProgress(pub f32);

/// Incoming explainer line. Applied to the visible [`LoadingScreen`].
#[derive(Message, Debug, Clone)]
pub struct LoadingExplainerText(pub String);

/// Marker on the spawned loading-screen root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct LoadingScreen;

impl LoadingScreen {
	pub fn scene(progress: f32, explainer: impl Into<String>) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> =
			vec![Box::new(LoadingStack::new(progress, explainer).scene())];
		bsn! {
			LoadingScreen
			MenuScreen
			MenuController
			Node {
				width: percent(100),
				height: percent(100),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
			}
			Pickable::IGNORE
			Children [ {children} ]
		}
	}
}

pub fn request_show_loading(commands: &mut Commands) {
	spawn_menu_show_request(commands, RequestShowLoading);
}

pub fn request_loading_progress(commands: &mut Commands, progress: f32) {
	commands.write_message(LoadingProgress(progress));
}

pub fn request_loading_explainer(commands: &mut Commands, explainer: impl Into<String>) {
	commands.write_message(LoadingExplainerText(explainer.into()));
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadingScreenSystems {
	Apply,
}

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.add_message::<LoadingProgress>()
			.add_message::<LoadingExplainerText>()
			.add_systems(
				Update,
				(apply_show_loading, apply_loading_progress, apply_loading_explainer)
					.chain()
					.in_set(LoadingScreenSystems::Apply),
			);
	}
}

fn apply_show_loading(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowLoading>>,
	existing: Query<Entity, With<MenuScreen>>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
		return;
	}
	commands.spawn_scene(LoadingScreen::scene(0.0, "Loading…"));
}

fn apply_loading_progress(
	mut updates: MessageReader<LoadingProgress>,
	screens: Query<Entity, With<LoadingScreen>>,
	children: Query<&Children>,
	mut fills: Query<&mut LoadingBarFill>,
) {
	let Some(latest) = updates.read().last() else {
		return;
	};
	for screen in &screens {
		set_loading_progress(screen, latest.0, &children, &mut fills);
	}
}

fn apply_loading_explainer(
	mut updates: MessageReader<LoadingExplainerText>,
	screens: Query<Entity, With<LoadingScreen>>,
	children: Query<&Children>,
	mut lines: Query<&mut Text, With<LoadingExplainer>>,
) {
	let Some(latest) = updates.read().last() else {
		return;
	};
	for screen in &screens {
		set_loading_explainer(screen, latest.0.clone(), &children, &mut lines);
	}
}
