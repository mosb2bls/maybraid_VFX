//! Placeholder telegraph and hazard preview. Gameplay still writes `Hit` if
//! this presenter is removed.
//!
//! `attacks-playground --capture` settles, casts the column, then cancels so
//! xvfb/lavapipe captures can record windup, execute hits, and cleanup.

use std::fs;
use std::path::PathBuf;

use attacks::{
	AimPolicy, AttackCatalog, AttackDefinition, AttackEnded, AttackId, AttackInstance,
	AttackPlaceholderPresentationPlugin, AttackStage, AttackUser, AttacksPlugin, BeginAttack,
	CancelAttacks, HazardOrigin, HazardShape, HazardSpec, TelegraphSpec,
};
use bevy::app::AppExit;
use bevy::prelude::*;
use damage::{apply_hits, DamageApplied, Died, Health};

const COLUMN: AttackId = AttackId("column");
const WAVE: AttackId = AttackId("wave");

fn main() {
	let capture = std::env::args().any(|arg| arg == "--capture");
	let mut app = App::new();
	app.add_plugins(DefaultPlugins.set(WindowPlugin {
		primary_window: Some(Window {
			title: "Maybraid Attack Placeholders".into(),
			resolution: (1280, 720).into(),
			decorations: !capture,
			resizable: false,
			..default()
		}),
		..default()
	}))
	.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.1)))
	.insert_resource(if capture { PreviewMode::Capture } else { PreviewMode::Loop })
	.insert_resource(CaptureIo::from_env())
	.insert_resource(PreviewStats::default())
	.add_plugins(AttacksPlugin)
	.add_plugins(AttackPlaceholderPresentationPlugin)
	.add_message::<DamageApplied>()
	.add_message::<Died>()
	.add_systems(PostUpdate, apply_hits)
	.add_systems(Startup, setup)
	.add_systems(
		Update,
		(retrigger, capture_sequence, cancel_on_key, track_hits, tint_target, update_hud),
	);
	if capture {
		app.add_systems(Startup, |io: Res<CaptureIo>| io.write("SETTLE"));
	}
	app.run();
}

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
enum PreviewMode {
	Loop,
	Capture,
}

#[derive(Resource)]
struct CaptureIo {
	state_path: PathBuf,
}

impl CaptureIo {
	fn from_env() -> Self {
		Self {
			state_path: std::env::var("ATTACKS_PREVIEW_STATE")
				.map(PathBuf::from)
				.unwrap_or_else(|_| PathBuf::from("/tmp/attacks-preview-state")),
		}
	}

	fn write(&self, line: &str) {
		if let Some(parent) = self.state_path.parent() {
			let _ = fs::create_dir_all(parent);
		}
		let _ = fs::write(&self.state_path, format!("{line}\n"));
		eprintln!("attacks-preview: {line}");
	}
}

#[derive(Resource, Default)]
struct PreviewStats {
	hits: u32,
	cancelled: bool,
}

#[derive(Resource)]
struct CapturePlan {
	phase: CapturePhase,
}

enum CapturePhase {
	Settle { remaining: f32 },
	AwaitWindup,
	AwaitHits,
	HoldExecute { remaining: f32 },
	AwaitCleanup,
	HoldCancelled { remaining: f32 },
}

#[derive(Component)]
struct PreviewCaster {
	next: AttackId,
	wait: f32,
}

#[derive(Component)]
struct PreviewTarget(Entity);

#[derive(Component)]
struct PreviewHud;

#[derive(Component)]
struct TargetBody;

fn setup(
	mut commands: Commands,
	mode: Res<PreviewMode>,
	mut catalog: ResMut<AttackCatalog>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	catalog
		.insert(
			AttackDefinition::with_authored_defaults(
				COLUMN,
				1.2,
				2.4,
				0.6,
				AimPolicy::TargetAtWindup,
			)
			.with_telegraph(TelegraphSpec {
				shape: HazardShape::Sphere { radius: 2.2 },
				origin: HazardOrigin::AlongAim { distance: 1.6 },
				persist_into_execute: false,
			})
			.authored_hazard(
				HazardShape::Cylinder { radius: 2.2, height: 7.0 },
				HazardOrigin::AlongAim { distance: 1.6 },
			),
		)
		.insert(
			AttackDefinition::with_authored_defaults(
				WAVE,
				1.0,
				2.0,
				0.5,
				AimPolicy::TrackUntilExecute,
			)
			.with_telegraph(TelegraphSpec {
				shape: HazardShape::Line { length: 10.0, half_width: 0.45 },
				origin: HazardOrigin::AimPoint,
				persist_into_execute: true,
			})
			.with_hazard(HazardSpec {
				shape: HazardShape::Wavefront { radius: 1.3, speed: 5.0, length: 10.0 },
				origin: HazardOrigin::AimPoint,
				damage: attacks::DamageSpec::authored(),
			}),
		);

	commands.spawn((
		Camera3d::default(),
		AmbientLight { brightness: 420.0, ..default() },
		Transform::from_xyz(9.5, 6.5, 9.0).looking_at(Vec3::new(0.0, 2.0, 3.0), Vec3::Y),
	));
	commands.spawn((
		DirectionalLight { illuminance: 8_000.0, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.6, 0.0)),
	));
	commands.spawn((
		Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(20.0)))),
		MeshMaterial3d(materials.add(Color::srgb(0.16, 0.18, 0.2))),
		Transform::from_xyz(0.0, 0.0, 0.0),
	));

	let owner = commands
		.spawn((
			Name::new("owner"),
			AttackUser::default(),
			PreviewCaster {
				next: COLUMN,
				wait: if *mode == PreviewMode::Capture { 1000.0 } else { 0.4 },
			},
			Mesh3d(meshes.add(Cuboid::new(1.2, 2.4, 1.2))),
			MeshMaterial3d(materials.add(Color::srgb(0.35, 0.4, 0.48))),
			Transform::from_xyz(0.0, 1.2, 4.2),
		))
		.id();
	let target = commands
		.spawn((
			Name::new("target"),
			TargetBody,
			Health::from_max(400.0),
			Mesh3d(meshes.add(Capsule3d::new(0.45, 1.3))),
			MeshMaterial3d(materials.add(Color::srgb(0.55, 0.72, 0.84))),
			Transform::from_xyz(0.0, 1.2, 2.4),
		))
		.id();
	commands.entity(owner).insert(PreviewTarget(target));

	commands.spawn((
		PreviewHud,
		Text::new(hud_text("idle", 400.0, 0, false)),
		TextFont { font_size: FontSize::Px(26.0), ..default() },
		TextColor(Color::srgb(0.95, 0.93, 0.86)),
		TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.75) },
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(18.0),
			left: Val::Px(18.0),
			..default()
		},
	));

	if *mode == PreviewMode::Capture {
		commands.insert_resource(CapturePlan { phase: CapturePhase::Settle { remaining: 4.0 } });
	}
}

fn retrigger(
	mode: Res<PreviewMode>,
	time: Res<Time>,
	mut casters: Query<(Entity, &mut PreviewCaster, &PreviewTarget, &AttackUser)>,
	mut begins: MessageWriter<BeginAttack>,
) {
	if *mode == PreviewMode::Capture {
		return;
	}
	let dt = time.delta_secs();
	for (owner, mut caster, target, user) in &mut casters {
		caster.wait -= dt;
		if caster.wait > 0.0 || !user.ready(caster.next) {
			continue;
		}
		begins.write(BeginAttack::new(owner, caster.next).with_target(target.0));
		caster.next = if caster.next == COLUMN { WAVE } else { COLUMN };
		caster.wait = 1.2;
	}
}

fn cancel_on_key(
	keys: Res<ButtonInput<KeyCode>>,
	casters: Query<Entity, With<PreviewCaster>>,
	mut cancels: MessageWriter<CancelAttacks>,
) {
	if !keys.just_pressed(KeyCode::KeyC) {
		return;
	}
	for owner in &casters {
		cancels.write(CancelAttacks::all(owner));
	}
}

#[allow(clippy::too_many_arguments)]
fn capture_sequence(
	mode: Res<PreviewMode>,
	time: Res<Time>,
	io: Res<CaptureIo>,
	stats: Res<PreviewStats>,
	instances: Query<&AttackInstance>,
	casters: Query<(Entity, &PreviewTarget), With<PreviewCaster>>,
	mut plan: Option<ResMut<CapturePlan>>,
	mut begins: MessageWriter<BeginAttack>,
	mut cancels: MessageWriter<CancelAttacks>,
	mut exit: MessageWriter<AppExit>,
) {
	if *mode != PreviewMode::Capture {
		return;
	}
	let Some(plan) = plan.as_mut() else {
		return;
	};
	let dt = time.delta_secs();
	let stage = instances.iter().next().map(|instance| instance.stage);
	match &mut plan.phase {
		CapturePhase::Settle { remaining } => {
			*remaining -= dt;
			if *remaining <= 0.0 {
				if let Ok((owner, target)) = casters.single() {
					begins.write(BeginAttack::new(owner, COLUMN).with_target(target.0));
				}
				plan.phase = CapturePhase::AwaitWindup;
			}
		}
		CapturePhase::AwaitWindup => {
			if stage == Some(AttackStage::Windup) {
				io.write("WINDUP");
				plan.phase = CapturePhase::AwaitHits;
			}
		}
		CapturePhase::AwaitHits => {
			if stage == Some(AttackStage::Execute) && stats.hits > 0 {
				io.write(&format!("EXECUTE hits={}", stats.hits));
				plan.phase = CapturePhase::HoldExecute { remaining: 0.7 };
			}
		}
		CapturePhase::HoldExecute { remaining } => {
			*remaining -= dt;
			if *remaining <= 0.0 {
				if let Ok((owner, _)) = casters.single() {
					cancels.write(CancelAttacks::all(owner));
				}
				plan.phase = CapturePhase::AwaitCleanup;
			}
		}
		CapturePhase::AwaitCleanup => {
			if stats.cancelled && instances.iter().next().is_none() {
				io.write("CANCELLED");
				plan.phase = CapturePhase::HoldCancelled { remaining: 0.7 };
			}
		}
		CapturePhase::HoldCancelled { remaining } => {
			*remaining -= dt;
			if *remaining <= 0.0 {
				io.write("DONE");
				exit.write(AppExit::Success);
			}
		}
	}
}

fn track_hits(
	mut applied: MessageReader<DamageApplied>,
	mut ended: MessageReader<AttackEnded>,
	mut stats: ResMut<PreviewStats>,
) {
	stats.hits += applied.read().count() as u32;
	if ended.read().any(|event| event.reason == attacks::AttackEndReason::Cancelled) {
		stats.cancelled = true;
	}
}

fn tint_target(
	health: Query<&Health, With<TargetBody>>,
	targets: Query<&MeshMaterial3d<StandardMaterial>, With<TargetBody>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let Ok(pool) = health.single() else {
		return;
	};
	let Ok(handle) = targets.single() else {
		return;
	};
	let Some(mut material) = materials.get_mut(&handle.0) else {
		return;
	};
	let hurt = 1.0 - pool.fraction();
	material.base_color = Color::srgb(0.55 + 0.4 * hurt, 0.72 - 0.5 * hurt, 0.84 - 0.6 * hurt);
}

fn update_hud(
	stats: Res<PreviewStats>,
	instances: Query<&AttackInstance>,
	health: Query<&Health, With<TargetBody>>,
	mut hud: Query<&mut Text, With<PreviewHud>>,
) {
	let Ok(mut text) = hud.single_mut() else {
		return;
	};
	let stage = if stats.cancelled && instances.iter().next().is_none() {
		"cancelled"
	} else {
		match instances.iter().next().map(|instance| instance.stage) {
			Some(AttackStage::Windup) => "windup",
			Some(AttackStage::Execute) => "execute",
			Some(AttackStage::Recover) => "recover",
			None => "idle",
		}
	};
	let hp = health.single().map(|pool| pool.current).unwrap_or(0.0);
	text.0 = hud_text(stage, hp, stats.hits, stats.cancelled);
}

fn hud_text(stage: &str, hp: f32, hits: u32, cancelled: bool) -> String {
	format!(
		"stage: {stage}\ntarget HP: {hp:.0} / 400\nhits: {hits}{}\nC cancels",
		if cancelled { "\ncleanup: bindings despawned" } else { "" }
	)
}
