//! Laufer / Tanzer mode plate: label plus Maybraid loading-track charge bar.

use bevy::prelude::*;
use item_icons::ItemIconCache;
use menu_components::{LoadingBarFill, TEXT_YELLOW, TEXT_YELLOW_FAINT};
use screen_capture::CaptureSubject;

use crate::callout::TanzCallout;
use crate::{ControlMode, TanzUser};

const PANEL: Color = Color::srgba(0.05, 0.05, 0.08, 0.55);
const INK: Color = Color::srgb(0.92, 0.9, 0.82);
const ICON_PX: f32 = 40.0;
/// Same proportions as the loading screen track, scaled for the corner HUD.
const MODE_BAR_WIDTH: f32 = 168.0;
const MODE_BAR_HEIGHT: f32 = 4.0;

#[derive(Component)]
pub struct TanzModeHud;

#[derive(Component)]
pub struct TanzModeHudLabel;

#[derive(Component)]
pub struct TanzModeHudBarFill;

#[derive(Component)]
pub struct TanzCalloutHud;

#[derive(Component)]
pub struct TanzCalloutHudPanel;

#[derive(Component)]
pub struct TanzCalloutHudIcon;

#[derive(Component)]
pub struct TanzCalloutHudLabel;

pub fn spawn_tanz_mode_hud(mut commands: Commands) {
	commands
		.spawn((
			Name::new("tanz-mode-hud"),
			TanzModeHud,
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(18.0),
				right: Val::Px(18.0),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(6.0),
				padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
				..default()
			},
			BackgroundColor(PANEL),
			GlobalZIndex(24),
			Pickable::IGNORE,
			Visibility::Hidden,
		))
		.with_children(|parent| {
			parent.spawn((
				TanzModeHudLabel,
				Text::new(ControlMode::Standard.mode_label()),
				TextFont { font_size: FontSize::Px(14.0), ..default() },
				TextColor(INK),
				Pickable::IGNORE,
			));
			parent
				.spawn((
					Node {
						width: Val::Px(MODE_BAR_WIDTH),
						height: Val::Px(MODE_BAR_HEIGHT),
						flex_shrink: 0.0,
						overflow: Overflow::clip(),
						..default()
					},
					BackgroundColor(TEXT_YELLOW_FAINT),
					Pickable::IGNORE,
				))
				.with_children(|track| {
					track.spawn((
						TanzModeHudBarFill,
						LoadingBarFill::new(0.0),
						Node {
							width: Val::Percent(0.0),
							height: Val::Percent(100.0),
							..default()
						},
						BackgroundColor(TEXT_YELLOW),
						Pickable::IGNORE,
					));
				});
		});
}

pub fn spawn_tanz_callout_hud(mut commands: Commands) {
	commands
		.spawn((
			Name::new("tanz-callout-hud"),
			TanzCalloutHud,
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(56.0),
				left: Val::Px(0.0),
				right: Val::Px(0.0),
				width: Val::Percent(100.0),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			GlobalZIndex(24),
			Pickable::IGNORE,
			Visibility::Hidden,
		))
		.with_children(|parent| {
			parent
				.spawn((
					TanzCalloutHudPanel,
					Node {
						flex_direction: FlexDirection::Row,
						align_items: AlignItems::Center,
						column_gap: Val::Px(10.0),
						padding: UiRect::axes(Val::Px(12.0), Val::Px(7.0)),
						..default()
					},
					BackgroundColor(PANEL),
					Pickable::IGNORE,
				))
				.with_children(|row| {
					row.spawn((
						TanzCalloutHudIcon,
						ImageNode { color: Color::NONE, ..default() },
						Node {
							width: Val::Px(ICON_PX),
							height: Val::Px(ICON_PX),
							flex_shrink: 0.0,
							display: Display::None,
							..default()
						},
						Pickable::IGNORE,
					));
					row.spawn((
						TanzCalloutHudLabel,
						Text::new(""),
						TextFont { font_size: FontSize::Px(14.0), ..default() },
						TextColor(INK),
						Pickable::IGNORE,
					));
				});
		});
}

pub fn sync_tanz_mode_hud(
	users: Query<&TanzUser>,
	mut roots: Query<&mut Visibility, With<TanzModeHud>>,
	mut labels: Query<&mut Text, With<TanzModeHudLabel>>,
	mut fills: Query<&mut LoadingBarFill, With<TanzModeHudBarFill>>,
) {
	let user = users.iter().next();
	for mut visibility in &mut roots {
		*visibility = if user.is_some() { Visibility::Visible } else { Visibility::Hidden };
	}
	let Some(user) = user else {
		return;
	};
	let label = user.mode.mode_label();
	for mut text in &mut labels {
		if text.0 != label {
			text.0 = String::from(label);
		}
	}
	for mut fill in &mut fills {
		fill.progress = user.toggle.progress;
	}
}

pub fn sync_tanz_callout_hud(
	callout: Res<TanzCallout>,
	icons: Option<Res<ItemIconCache>>,
	mut roots: Query<&mut Visibility, With<TanzCalloutHud>>,
	mut panels: Query<&mut BackgroundColor, With<TanzCalloutHudPanel>>,
	mut image_nodes: Query<(&mut ImageNode, &mut Node), With<TanzCalloutHudIcon>>,
	mut labels: Query<(&mut Text, &mut TextColor), With<TanzCalloutHudLabel>>,
) {
	let visible = callout.current.is_some();
	for mut visibility in &mut roots {
		*visibility = if visible { Visibility::Visible } else { Visibility::Hidden };
	}
	let Some(view) = &callout.current else {
		return;
	};
	let alpha = callout.alpha();
	for mut color in &mut panels {
		color.0 = Color::srgba(0.05, 0.05, 0.08, 0.55 * alpha);
	}
	let handle = icons
		.as_ref()
		.and_then(|cache| cache.silhouette(&crate::TanzPose(view.id).capture_id()));
	for (mut image, mut node) in &mut image_nodes {
		if let Some(handle) = handle.clone() {
			image.image = handle;
			image.color = Color::srgba(0.92, 0.9, 0.82, alpha);
			node.display = Display::Flex;
		} else {
			image.color = Color::NONE;
			node.display = Display::None;
		}
	}
	for (mut text, mut color) in &mut labels {
		if text.0 != view.name {
			text.0 = String::from(view.name);
		}
		color.0 = Color::srgba(0.92, 0.9, 0.82, alpha);
	}
}

#[cfg(test)]
mod tests {
	use crate::stack::TanzId;
	use crate::TanzPose;
	use item_icons::ItemIconCache;
	use screen_capture::{CaptureId, CaptureMode, CaptureSubject};

	#[test]
	fn callout_icon_stays_hidden_until_the_cache_has_a_handle() {
		let cache = ItemIconCache::default();
		assert!(cache.silhouette(&TanzPose(TanzId::ForwardDash).capture_id()).is_none());
		assert!(!cache.failed(&CaptureId::new("unused"), CaptureMode::Silhouette));
	}
}
