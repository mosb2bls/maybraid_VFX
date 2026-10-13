//! Frame timing diagnostics for the vegetation-on-terrain playground.
//!
//! Toggle with env `WORLD_TERRAIN_DIAG` (comma-separated):
//! - `fps` — throttled `[timing]` FPS / frame_ms; playgrounds also show a HUD
//! - `off` — disable (default when unset)
//!
//! The game shell inserts [`PlaygroundDiag`] `{ fps: true, hud: false }` so the
//! log stays on without the overlay.
//!
//! Examples:
//! ```text
//! WORLD_TERRAIN_DIAG=fps
//! WORLD_TERRAIN_DIAG=off   # default
//! ```

use std::time::Duration;

use bevy::camera::visibility::VisibilitySystems;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use game_commands::ui::GameCommandStatusText;
use lod::LodSceneHost;
use vegetation_components::{FoliageLodProbe, StickLodProbe};

use crate::commands::RequestMeshStats;
use crate::ui;

const ENV_DIAG: &str = "WORLD_TERRAIN_DIAG";
const LOG_INTERVAL: Duration = Duration::from_secs(1);

/// Parsed [`WORLD_TERRAIN_DIAG`] flags.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaygroundDiag {
	pub fps: bool,
	/// On-screen FPS overlay. The game shell leaves this off.
	pub hud: bool,
}

impl Default for PlaygroundDiag {
	fn default() -> Self {
		Self::from_env()
	}
}

impl PlaygroundDiag {
	pub fn from_env() -> Self {
		let raw = std::env::var(ENV_DIAG).unwrap_or_default();
		let raw = raw.trim();
		if raw.is_empty() {
			return Self { fps: false, hud: false };
		}
		let mut fps = false;
		let mut off = false;
		for part in raw.split(',') {
			let part = part.trim();
			if part.is_empty() {
				continue;
			}
			match part.to_ascii_lowercase().as_str() {
				"off" | "none" | "0" => off = true,
				"fps" | "timing" | "on" => fps = true,
				other => {
					eprintln!("[{ENV_DIAG}] unknown flag {other:?} (use fps|off)");
				}
			}
		}
		if off {
			return Self { fps: false, hud: false };
		}
		Self { fps, hud: fps }
	}

	pub fn summary(self) -> String {
		if self.fps {
			format!("{ENV_DIAG}=fps")
		} else {
			format!("{ENV_DIAG}=off")
		}
	}
}

#[derive(Component)]
struct FrameHudRoot;

#[derive(Component)]
struct FrameHudText;

/// Toggle FPS log + HUD (`/stats fps` in the world playground).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestFpsToggle;

/// Count total vs view-visible mesh triangles after visibility is resolved.
pub struct MeshStatsPlugin;

impl Plugin for MeshStatsPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PostUpdate, apply_mesh_stats.after(VisibilitySystems::CheckVisibility));
	}
}

/// Count total vs view-visible mesh triangles (`ViewVisibility`) and LOD probe hosts.
fn apply_mesh_stats(
	mut commands: Commands,
	mut status: Option<ResMut<GameCommandStatusText>>,
	mesh_assets: Res<Assets<Mesh>>,
	requests: Query<Entity, With<RequestMeshStats>>,
	mesh_entities: Query<(&Mesh3d, &ViewVisibility)>,
	foliage_probes: Query<(), With<FoliageLodProbe>>,
	stick_probes: Query<(), With<StickLodProbe>>,
	lod_hosts: Query<(), With<LodSceneHost>>,
) {
	for entity in &requests {
		let mut total_entities = 0usize;
		let mut visible_entities = 0usize;
		let mut missing = 0usize;
		let mut total_tris = 0usize;
		let mut visible_tris = 0usize;
		let mut unique_handles = std::collections::HashSet::new();
		let mut visible_unique_handles = std::collections::HashSet::new();

		for (mesh3d, view_visibility) in &mesh_entities {
			total_entities += 1;
			unique_handles.insert(mesh3d.0.id());
			let Some(mesh) = mesh_assets.get(&mesh3d.0) else {
				missing += 1;
				continue;
			};
			let verts = mesh.count_vertices();
			let index_count = mesh.indices().map(|i| i.len()).unwrap_or(verts);
			let tris = index_count / 3;
			total_tris += tris;
			if view_visibility.get() {
				visible_entities += 1;
				visible_unique_handles.insert(mesh3d.0.id());
				visible_tris += tris;
			}
		}

		let foliage_probes = foliage_probes.iter().count();
		let stick_probes = stick_probes.iter().count();
		let lod_hosts = lod_hosts.iter().count();
		let probes_total = foliage_probes + stick_probes;

		let text = format!(
			"stats mesh:\n  total_tris={total_tris}\n  visible_tris={visible_tris}\n  entities={total_entities} visible_entities={visible_entities} unique_handles={} visible_unique={} missing={missing}\n  probes: foliage={foliage_probes} stick={stick_probes} total={probes_total}\n  lod_hosts={lod_hosts}",
			unique_handles.len(),
			visible_unique_handles.len(),
		);
		info!("{text}");
		ui::write_status(&mut status, text);
		commands.entity(entity).despawn();
	}
}

pub struct PlaygroundTimingPlugin;

impl Plugin for PlaygroundTimingPlugin {
	fn build(&self, app: &mut App) {
		if !app.world().contains_resource::<PlaygroundDiag>() {
			app.insert_resource(PlaygroundDiag::from_env());
		}
		if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
			app.add_plugins(FrameTimeDiagnosticsPlugin::default());
		}
		app.add_systems(Startup, spawn_frame_hud)
			.add_systems(Update, (toggle_fps_logging, log_frame_timing, update_frame_hud));
	}
}

pub fn toggle_fps_logging(
	mut commands: Commands,
	mut diag: ResMut<PlaygroundDiag>,
	mut status: Option<ResMut<game_commands::ui::GameCommandStatusText>>,
	requests: Query<Entity, With<RequestFpsToggle>>,
) {
	for entity in &requests {
		diag.fps = !diag.fps;
		let line = if diag.fps { "[timing] fps on" } else { "[timing] fps off" };
		if let Some(status) = status.as_mut() {
			status.0 = line.into();
		}
		info!("{line}");
		commands.entity(entity).despawn();
	}
}

fn spawn_frame_hud(mut commands: Commands, diag: Res<PlaygroundDiag>) {
	if !diag.hud {
		return;
	}
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(8.0),
				right: Val::Px(8.0),
				padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.05, 0.08, 0.12, 0.78)),
			Visibility::Hidden,
			FrameHudRoot,
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new("fps —"),
				TextFont { font_size: bevy::text::FontSize::Px(14.0), ..default() },
				TextColor(Color::srgb(0.92, 0.96, 1.0)),
				FrameHudText,
			));
		});
}

fn update_frame_hud(
	diag: Res<PlaygroundDiag>,
	diagnostics: Res<DiagnosticsStore>,
	mut root: Query<&mut Visibility, With<FrameHudRoot>>,
	mut text: Query<&mut Text, With<FrameHudText>>,
) {
	let Ok(mut visibility) = root.single_mut() else {
		return;
	};
	if !diag.fps || !diag.hud {
		*visibility = Visibility::Hidden;
		return;
	}
	*visibility = Visibility::Visible;
	let Ok(mut hud) = text.single_mut() else {
		return;
	};
	let fps = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FPS)
		.and_then(|d| d.smoothed())
		.unwrap_or(f64::NAN);
	let frame_ms = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
		.and_then(|d| d.smoothed())
		.unwrap_or(f64::NAN);
	*hud = Text::new(format!("fps {fps:.0}   {frame_ms:.1} ms"));
}

fn log_frame_timing(
	time: Res<Time>,
	diag: Res<PlaygroundDiag>,
	mut accum: Local<Duration>,
	diagnostics: Res<DiagnosticsStore>,
) {
	if !diag.fps {
		return;
	}
	*accum += time.delta();
	if *accum < LOG_INTERVAL {
		return;
	}
	*accum = Duration::ZERO;

	let fps = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FPS)
		.and_then(|d| d.smoothed())
		.unwrap_or(f64::NAN);
	let frame_ms = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
		.and_then(|d| d.smoothed())
		.unwrap_or(f64::NAN);
	eprintln!("[timing] fps={fps:.1} frame_ms={frame_ms:.2}");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn summary_names_fps_flag() {
		assert!(PlaygroundDiag { fps: true, hud: true }.summary().contains("fps"));
		assert!(PlaygroundDiag { fps: false, hud: false }.summary().contains("off"));
	}
}
