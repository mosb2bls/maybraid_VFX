//! Lubikan HUD fixture preview. `--capture` writes PNGs to `/opt/cursor/artifacts/`.

use std::path::{Path, PathBuf};

use bevy::{
	prelude::*,
	render::view::screenshot::{save_to_disk, Capturing, Screenshot},
	winit::WinitSettings,
};
use clap::Parser;
use combat_hud::{CombatHudInventoryCycleCategory, CombatHudPlugin, CombatHudVisible};
use damage::Health;
use lubikans::{
	fixtures::{install_interface, spawn_self_owner},
	LubikanActivationPhase, LubikanCatalog, LubikanChargeChanged, LubikanChargeReason,
	LubikanInterfacePlugin, LubikanLedger, LubikanOwner,
};
use player::Player;

#[derive(Parser, Debug, Resource)]
#[command(name = "combat-hud-playground")]
struct Cli {
	#[arg(long)]
	capture: bool,
	#[arg(long)]
	points: bool,
}

#[derive(Resource, Clone, Debug)]
struct PreviewControls {
	label: &'static str,
	width: f32,
	height: f32,
	charge: f32,
	activation: LubikanActivationPhase,
	hud_visible: bool,
	lubikan_category: bool,
	emit_changes: Vec<(f32, Option<Vec3>)>,
}

#[derive(Resource, Default)]
struct CapturePlan {
	steps: Vec<PreviewControls>,
	index: usize,
	dwell: f32,
	emitted_step: Option<usize>,
}

#[derive(Resource)]
struct PreviewOwner(Entity);

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let cli = Cli::parse();
	let capture = cli.capture;
	let assets_path = assets_root();
	let mut app = App::new();
	app.insert_resource(cli)
		.add_plugins(DefaultPlugins.set(AssetPlugin {
			file_path: assets_path.to_string_lossy().into(),
			..default()
		}).set(WindowPlugin {
			primary_window: Some(Window {
				title: String::from("combat-hud-playground"),
				resolution: (1280, 720).into(),
				..default()
			}),
			..default()
		}))
		.insert_resource(WinitSettings::desktop_app())
		.insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)))
		.add_plugins(LubikanInterfacePlugin)
		.add_plugins(CombatHudPlugin {
			health_bars: false,
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			lubikan_charge: true,
			lubikan_point_feedback: true,
			score: false,
		})
		.insert_resource(CombatHudVisible(true))
		.add_systems(Update, (apply_preview, run_capture, finish_capture));
	install_interface(&mut app);
	let owner = spawn_self_owner(app.world_mut());
	app.world_mut().entity_mut(owner).insert((
		Player,
		Health { current: 72.0, max: 100.0 },
		Name::new("Preview"),
	));
	app.world_mut().spawn((
		Camera3d::default(),
		Transform::from_xyz(0.0, 6.0, 14.0).looking_at(Vec3::ZERO, Vec3::Y),
	));
	app.insert_resource(PreviewOwner(owner));
	if capture {
		app.insert_resource(build_capture_plan(app.world().resource::<Cli>()));
	}
	app.run();
	if capture {
		info!("capture sequence finished");
	}
}

fn build_capture_plan(cli: &Cli) -> CapturePlan {
	let mut steps = vec![
		step("normal", 1280.0, 720.0, 40.0, LubikanActivationPhase::Idle, true, false, vec![]),
		step("ready", 1280.0, 720.0, 90.0, LubikanActivationPhase::Idle, true, false, vec![]),
		step("healing", 1280.0, 720.0, 60.0, LubikanActivationPhase::Active, true, false, vec![]),
		step("empty", 1280.0, 720.0, 0.0, LubikanActivationPhase::Idle, true, false, vec![]),
		step("hidden", 1280.0, 720.0, 50.0, LubikanActivationPhase::Idle, false, false, vec![]),
		step("respawn", 390.0, 844.0, 10.0, LubikanActivationPhase::Idle, true, false, vec![]),
		step("category-cue", 1280.0, 720.0, 55.0, LubikanActivationPhase::Idle, true, true, vec![]),
	];
	if cli.points {
		steps.extend([
			step(
				"points-award",
				1280.0,
				720.0,
				55.0,
				LubikanActivationPhase::Idle,
				true,
				false,
				vec![
					(3.0, Some(Vec3::new(4.0, 1.0, -8.0))),
					(2.0, Some(Vec3::new(-12.0, 0.5, 3.0))),
				],
			),
			step(
				"points-penalty",
				1280.0,
				720.0,
				55.0,
				LubikanActivationPhase::Idle,
				true,
				false,
				vec![(-4.0, Some(Vec3::new(0.0, 2.0, 6.0)))],
			),
			step(
				"points-offscreen",
				1280.0,
				720.0,
				55.0,
				LubikanActivationPhase::Idle,
				true,
				false,
				vec![(5.0, Some(Vec3::new(200.0, 0.0, 0.0)))],
			),
		]);
	}
	CapturePlan { steps, index: 0, dwell: 0.0, emitted_step: None }
}

fn step(
	label: &'static str,
	width: f32,
	height: f32,
	charge: f32,
	activation: LubikanActivationPhase,
	hud_visible: bool,
	lubikan_category: bool,
	emit_changes: Vec<(f32, Option<Vec3>)>,
) -> PreviewControls {
	PreviewControls {
		label,
		width,
		height,
		charge,
		activation,
		hud_visible,
		lubikan_category,
		emit_changes,
	}
}

fn apply_preview(
	cli: Res<Cli>,
	mut plan: Option<ResMut<CapturePlan>>,
	owner: Res<PreviewOwner>,
	catalog: Res<LubikanCatalog>,
	mut hud: ResMut<CombatHudVisible>,
	mut category: ResMut<CombatHudInventoryCycleCategory>,
	mut windows: Query<&mut Window>,
	mut ledgers: Query<&mut LubikanLedger, With<LubikanOwner>>,
	mut commands: Commands,
	time: Res<Time>,
) {
	if !cli.capture {
		let t = time.elapsed_secs();
		let phase = (t / 2.0).floor() as i32 % 4;
		if let Ok(mut ledger) = ledgers.get_mut(owner.0) {
			let target = match phase {
				0 => 40.0,
				1 => 95.0,
				2 => 0.0,
				_ => 55.0,
			};
			let delta = target - ledger.equipped().charge().current();
			ledger.apply_to_equipped(delta);
			let account = ledger.equipped().id();
			ledger.reset_activation(account);
			if phase == 2 {
				ledger.try_activate(&catalog, None).ok();
			}
		}
		*category = if phase == 3 {
			CombatHudInventoryCycleCategory::Lubikans
		} else {
			CombatHudInventoryCycleCategory::Weapons
		};
		return;
	}
	let Some(mut plan) = plan else { return };
	let index = plan.index;
	let Some(step) = plan.steps.get(index).cloned() else { return };
	if let Ok(mut ledger) = ledgers.get_mut(owner.0) {
		let current = ledger.equipped().charge().current();
		ledger.apply_to_equipped(step.charge - current);
		let account = ledger.equipped().id();
		ledger.reset_activation(account);
		match step.activation {
			LubikanActivationPhase::Idle => {}
			LubikanActivationPhase::Active => {
				ledger.try_activate(&catalog, None).ok();
			}
			LubikanActivationPhase::Completed => {
				ledger.try_activate(&catalog, None).ok();
				ledger.complete_activation(ledger.equipped().id());
			}
		}
		let account = ledgers
			.get(owner.0)
			.map(|ledger| ledger.equipped().id())
			.unwrap_or(lubikans::LubikanAccountId::SELF);
		if plan.emitted_step != Some(index) {
			for (amount, origin) in &step.emit_changes {
				commands.write_message(LubikanChargeChanged::new(
					owner.0,
					account,
					*amount,
					if *amount < 0.0 {
						LubikanChargeReason::ADJUSTMENT
					} else {
						LubikanChargeReason::COMBAT_HIT
					},
					*origin,
				));
			}
			plan.emitted_step = Some(index);
		}
	}
	*hud = CombatHudVisible(step.hud_visible);
	*category = if step.lubikan_category {
		CombatHudInventoryCycleCategory::Lubikans
	} else {
		CombatHudInventoryCycleCategory::Weapons
	};
	for mut window in &mut windows {
		window.resolution.set_physical_resolution(step.width as u32, step.height as u32);
	}
}

fn run_capture(
	time: Res<Time>,
	cli: Res<Cli>,
	mut plan: Option<ResMut<CapturePlan>>,
	mut commands: Commands,
	capturing: Query<Entity, With<Capturing>>,
) {
	if !cli.capture {
		return;
	}
	let Some(mut plan) = plan else { return };
	if plan.index >= plan.steps.len() {
		return;
	}
	let _ = std::fs::create_dir_all("/opt/cursor/artifacts");
	if !capturing.is_empty() {
		return;
	}
	plan.dwell += time.delta_secs();
	if plan.dwell < 0.45 {
		return;
	}
	plan.dwell = 0.0;
	let label = plan.steps[plan.index].label;
	let path = format!("/opt/cursor/artifacts/lubikan-hud-{}.png", label);
	if std::env::var_os("COMBAT_HUD_SCROT").is_some() {
		let _ = std::process::Command::new("scrot").arg(&path).status();
		plan.index += 1;
		plan.emitted_step = None;
	} else {
		commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
		plan.index += 1;
		plan.emitted_step = None;
	}
}

fn finish_capture(
	cli: Res<Cli>,
	plan: Option<Res<CapturePlan>>,
	capturing: Query<Entity, With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut cooldown: Local<u8>,
) {
	if !cli.capture {
		return;
	}
	let Some(plan) = plan else { return };
	if plan.index < plan.steps.len() {
		*cooldown = 0;
		return;
	}
	if capturing.is_empty() {
		*cooldown = cooldown.saturating_add(1);
		if *cooldown >= 4 {
			exit.write(AppExit::Success);
		}
	}
}
