//! Lubikan HUD fixture preview.
//!
//! `capture-states.sh` drives `--capture-states` with ImageMagick `import -window`.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::winit::WinitSettings;
use clap::Parser;
use combat_hud::{
	idle_cycle_outline, CombatHudCyclePanel, CombatHudInventoryCycleCategory, CombatHudPlugin,
	CombatHudVisible,
};
use damage::Health;
use lubikans::{
	fixtures::{install_gameplay, install_interface, spawn_self_owner},
	LubikanActivationRequest, LubikanCatalog, LubikanChargeChanged, LubikanChargeReason,
	LubikanHealingEnabled, LubikanInterfacePlugin, LubikanLedger, LubikanOwner, LubikanSelfHeal,
};
use player::Player;

const CAPTURE_WIDTH: u32 = 800;
const CAPTURE_HEIGHT: u32 = 450;
const WINDOW_TITLE: &str = "combat-hud-playground";

#[derive(Parser, Debug, Resource)]
#[command(name = "combat-hud-playground")]
struct Cli {
	/// Automated state machine for `capture-states.sh` (six PR preview frames).
	#[arg(long)]
	capture_states: bool,
	/// Three frames: teal outline on the D-pad-selected system panel.
	#[arg(long)]
	capture_cycle: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StateKind {
	PartialCharge,
	Ready,
	Activating,
	SelfHealing,
	PointMidflight,
	PausedHidden,
	CycleWeapons,
	CycleSkillMaps,
	CycleLubikans,
}

#[derive(Clone, Copy, Debug)]
struct StateStep {
	label: &'static str,
	kind: StateKind,
	/// Seconds to hold before writing `.capture-ready` for `import`.
	dwell_secs: f32,
}

#[derive(Resource)]
struct StateCapturePlan {
	steps: Vec<StateStep>,
	index: usize,
	dwell: f32,
	last_applied: Option<usize>,
	captured_this_step: bool,
	ledger_snapshot: Option<f32>,
}

#[derive(Resource, Default)]
struct CaptureSelfHealPriming {
	armed: bool,
}

#[derive(Resource)]
struct PreviewOwner(Entity);

fn artifacts_dir() -> PathBuf {
	std::env::var("ARTIFACTS_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|_| PathBuf::from("/opt/cursor/artifacts"))
}

fn capture_png_path(label: &str) -> PathBuf {
	if label.starts_with("cycle-") {
		artifacts_dir().join(format!("{label}.png"))
	} else {
		artifacts_dir().join(format!("lubikan-hud-state-{label}.png"))
	}
}

fn import_primary_window(label: &str) -> Result<(), String> {
	let out = capture_png_path(label);
	let wid_bytes = std::process::Command::new("xdotool")
		.args(["search", "--name", WINDOW_TITLE])
		.output()
		.map_err(|e| e.to_string())?;
	if !wid_bytes.status.success() {
		return Err(format!(
			"xdotool search failed: {}",
			String::from_utf8_lossy(&wid_bytes.stderr)
		));
	}
	let wid_text = String::from_utf8_lossy(&wid_bytes.stdout);
	let wid = wid_text.lines().next().unwrap_or("").trim();
	if wid.is_empty() {
		return Err("no window id".into());
	}
	let geom = std::process::Command::new("xwininfo")
		.args(["-id", wid])
		.output()
		.map_err(|e| e.to_string())?;
	let geom_text = String::from_utf8_lossy(&geom.stdout);
	let geometry = geom_text
		.lines()
		.find_map(|line| line.trim().strip_prefix("-geometry").map(|rest| rest.trim()))
		.ok_or("missing -geometry")?;
	let (w, h) = geometry
		.split_once('x')
		.and_then(|(w, rest)| rest.split_once('+').map(|(h, _)| (w, h)))
		.ok_or("bad geometry")?;
	let crop = format!("{w}x{h}+0+0");
	let out_path = out.to_string_lossy();
	let status = std::process::Command::new("import")
		.args(["-window", wid, "-crop", &crop, out_path.as_ref()])
		.status()
		.map_err(|e| e.to_string())?;
	if !status.success() {
		return Err("import failed".into());
	}
	info!("captured {} ({crop})", out.display());
	Ok(())
}

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn capture_steps() -> Vec<StateStep> {
	vec![
		StateStep {
			label: "01-partial-above-health",
			kind: StateKind::PartialCharge,
			dwell_secs: 0.35,
		},
		StateStep { label: "02-ready", kind: StateKind::Ready, dwell_secs: 0.35 },
		StateStep { label: "03-activating", kind: StateKind::Activating, dwell_secs: 0.35 },
		StateStep { label: "04-self-healing", kind: StateKind::SelfHealing, dwell_secs: 1.0 },
		StateStep {
			label: "05-point-midflight",
			kind: StateKind::PointMidflight,
			dwell_secs: 0.32,
		},
		StateStep { label: "06-paused-hidden", kind: StateKind::PausedHidden, dwell_secs: 0.35 },
	]
}

fn cycle_capture_steps() -> Vec<StateStep> {
	vec![
		StateStep { label: "cycle-01-weapons", kind: StateKind::CycleWeapons, dwell_secs: 2.4 },
		StateStep {
			label: "cycle-02-skill-maps",
			kind: StateKind::CycleSkillMaps,
			dwell_secs: 0.8,
		},
		StateStep { label: "cycle-03-lubikans", kind: StateKind::CycleLubikans, dwell_secs: 0.8 },
	]
}

fn spawn_cycle_standin_plates(mut commands: Commands) {
	spawn_standin_plate(
		&mut commands,
		"weapons-cycle-standin",
		CombatHudInventoryCycleCategory::Weapons,
		"Weapons",
		Val::Px(260.0),
		120.0,
	);
	spawn_standin_plate(
		&mut commands,
		"skill-map-cycle-standin",
		CombatHudInventoryCycleCategory::SkillMaps,
		"Skill Map",
		Val::Px(16.0),
		168.0,
	);
}

fn spawn_standin_plate(
	commands: &mut Commands,
	name: &'static str,
	category: CombatHudInventoryCycleCategory,
	caption: &'static str,
	bottom: Val,
	height: f32,
) {
	commands
		.spawn((
			Name::new(name),
			CombatHudCyclePanel(category),
			idle_cycle_outline(),
			Node {
				position_type: PositionType::Absolute,
				right: Val::Px(16.0),
				bottom,
				width: Val::Px(228.0),
				height: Val::Px(height),
				padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
				border: UiRect::all(Val::Px(1.5)),
				border_radius: BorderRadius::all(Val::Px(8.0)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(Color::srgba(0.52, 0.52, 0.56, 0.82)),
			BorderColor::all(Color::srgba(0.78, 0.78, 0.82, 0.9)),
			Visibility::Visible,
			Pickable::IGNORE,
			GlobalZIndex(i32::MAX - 7),
		))
		.with_children(|plate| {
			plate.spawn((
				Text::new(caption),
				TextFont { font_size: FontSize::Px(16.0), ..default() },
				TextColor(Color::srgb(1.0, 0.86, 0.22)),
				Pickable::IGNORE,
			));
		});
}

fn main() {
	let cli = Cli::parse();
	let assets_path = assets_root();
	let mut app = App::new();
	app.insert_resource(cli)
		.add_plugins(
			DefaultPlugins
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() })
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: WINDOW_TITLE.into(),
						resolution: (CAPTURE_WIDTH, CAPTURE_HEIGHT).into(),
						resizable: false,
						..default()
					}),
					..default()
				}),
		)
		// Continuous updates under xvfb (desktop_app stalls without input events).
		.insert_resource(WinitSettings::game())
		.insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)))
		.add_plugins(LubikanInterfacePlugin)
		.add_plugins(CombatHudPlugin {
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			lubikan_charge: true,
			lubikan_point_feedback: true,
			score: false,
		})
		.insert_resource(CombatHudVisible(true))
		.insert_resource(LubikanHealingEnabled(true))
		.init_resource::<CaptureSelfHealPriming>()
		.add_systems(Update, freeze_self_heal_capture);
	install_interface(&mut app);
	install_gameplay(&mut app);
	let owner = spawn_self_owner(app.world_mut());
	// No `Name`: vitals show the default character label ("Player"), not a debug title.
	app.world_mut()
		.entity_mut(owner)
		.insert((Player, Health { current: 68.0, max: 100.0 }));
	app.world_mut().spawn((
		Camera3d::default(),
		Transform::from_xyz(0.0, 6.0, 14.0).looking_at(Vec3::ZERO, Vec3::Y),
	));
	app.insert_resource(PreviewOwner(owner));
	app.add_systems(Update, (apply_capture_state, drive_capture_states).chain());
	let capture_states = app.world().resource::<Cli>().capture_states;
	let capture_cycle = app.world().resource::<Cli>().capture_cycle;
	if capture_cycle {
		app.add_systems(Startup, spawn_cycle_standin_plates);
	}
	if capture_states || capture_cycle {
		let _ = std::fs::create_dir_all(artifacts_dir());
		app.insert_resource(StateCapturePlan {
			steps: if capture_cycle { cycle_capture_steps() } else { capture_steps() },
			index: 0,
			dwell: 0.0,
			last_applied: None,
			captured_this_step: false,
			ledger_snapshot: None,
		});
	}
	app.run();
}

fn freeze_self_heal_capture(
	owner: Res<PreviewOwner>,
	mut priming: ResMut<CaptureSelfHealPriming>,
	heal: Res<LubikanSelfHeal>,
	mut enabled: ResMut<LubikanHealingEnabled>,
) {
	if !priming.armed {
		return;
	}
	if heal.active && heal.owner == Some(owner.0) {
		enabled.0 = false;
		priming.armed = false;
	}
}

#[allow(clippy::too_many_arguments)]
fn apply_capture_state(
	cli: Res<Cli>,
	plan: Option<ResMut<StateCapturePlan>>,
	owner: Res<PreviewOwner>,
	catalog: Res<LubikanCatalog>,
	mut hud: ResMut<CombatHudVisible>,
	mut priming: ResMut<CaptureSelfHealPriming>,
	mut healing_enabled: ResMut<LubikanHealingEnabled>,
	mut category: ResMut<CombatHudInventoryCycleCategory>,
	mut ledgers: Query<&mut LubikanLedger, With<LubikanOwner>>,
	mut healths: Query<&mut Health>,
	mut commands: Commands,
) {
	if !cli.capture_states && !cli.capture_cycle {
		return;
	}
	let Some(mut plan) = plan else { return };
	let index = plan.index;
	if index >= plan.steps.len() {
		return;
	}
	if plan.last_applied == Some(index) {
		return;
	}
	plan.last_applied = Some(index);
	let step = plan.steps[index];
	priming.armed = false;
	healing_enabled.0 = true;
	*category = match step.kind {
		StateKind::CycleSkillMaps => CombatHudInventoryCycleCategory::SkillMaps,
		StateKind::CycleLubikans => CombatHudInventoryCycleCategory::Lubikans,
		_ => CombatHudInventoryCycleCategory::Weapons,
	};
	*hud = CombatHudVisible(true);

	if let Ok(mut health) = healths.get_mut(owner.0) {
		*health = Health { current: 68.0, max: 100.0 };
	}

	if let Ok(mut ledger) = ledgers.get_mut(owner.0) {
		let account = ledger.equipped().id();
		ledger.reset_activation(account);
		let capacity = ledger.equipped().charge().capacity();
		let current = ledger.equipped().charge().current();
		match step.kind {
			StateKind::PartialCharge => {
				ledger.apply_to_equipped(42.0 - current);
			}
			StateKind::Ready => {
				ledger.apply_to_equipped(capacity - current);
			}
			StateKind::Activating => {
				ledger.apply_to_equipped(capacity - current);
				ledger.try_activate(&catalog, None).ok();
			}
			StateKind::SelfHealing => {
				if let Ok(mut health) = healths.get_mut(owner.0) {
					health.current = 40.0;
					health.max = 100.0;
				}
				ledger.apply_to_equipped(capacity - current);
				commands.write_message(LubikanActivationRequest::equipped(owner.0));
				priming.armed = true;
			}
			StateKind::PointMidflight => {
				ledger.apply_to_equipped(50.0 - current);
				commands.write_message(LubikanChargeChanged::new(
					owner.0,
					account,
					3.0,
					LubikanChargeReason::COMBAT_HIT,
					Some(Vec3::new(6.0, 1.0, -10.0)),
				));
			}
			StateKind::PausedHidden => {
				let charge = 55.0;
				ledger.apply_to_equipped(charge - current);
				plan.ledger_snapshot = Some(charge);
				*hud = CombatHudVisible(false);
			}
			StateKind::CycleWeapons | StateKind::CycleSkillMaps | StateKind::CycleLubikans => {
				ledger.apply_to_equipped(48.0 - current);
			}
		}
	}
}

#[allow(clippy::too_many_arguments)]
fn drive_capture_states(
	time: Res<Time>,
	cli: Res<Cli>,
	plan: Option<ResMut<StateCapturePlan>>,
	priming: Res<CaptureSelfHealPriming>,
	heal: Res<LubikanSelfHeal>,
	owner: Res<PreviewOwner>,
	ledgers: Query<&LubikanLedger, With<LubikanOwner>>,
	mut exit: MessageWriter<AppExit>,
) {
	if !cli.capture_states && !cli.capture_cycle {
		return;
	}
	let Some(mut plan) = plan else { return };

	if plan.index >= plan.steps.len() {
		exit.write(AppExit::Success);
		return;
	}

	plan.dwell += time.delta_secs();
	let step = plan.steps[plan.index];

	if plan.captured_this_step {
		if plan.dwell < 0.08 {
			return;
		}
		plan.index += 1;
		plan.dwell = 0.0;
		plan.captured_this_step = false;
		plan.last_applied = None;
		return;
	}

	if plan.dwell < step.dwell_secs {
		return;
	}
	if step.kind == StateKind::SelfHealing && (priming.armed || !heal.active) {
		return;
	}

	if let Err(err) = import_primary_window(step.label) {
		warn!("capture failed for {}: {}", step.label, err);
	} else if step.kind == StateKind::PausedHidden {
		let ledger_charge = ledgers
			.get(owner.0)
			.map(|ledger| ledger.equipped().charge().current())
			.unwrap_or(-1.0);
		let snapshot = plan.ledger_snapshot.unwrap_or(0.0);
		info!("paused snapshot ledger_charge={:.0} expected={:.0}", ledger_charge, snapshot);
	}
	plan.captured_this_step = true;
	plan.dwell = 0.0;
}
