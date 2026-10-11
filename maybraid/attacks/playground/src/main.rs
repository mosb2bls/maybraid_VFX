//! Placeholder telegraph and hazard preview. Gameplay still writes `Hit` if
//! this presenter is removed.

use attacks::{
	AimPolicy, AttackCatalog, AttackDefinition, AttackId, AttackPlaceholderPresentationPlugin,
	AttackUser, AttacksPlugin, BeginAttack, HazardOrigin, HazardShape, HazardSpec, TelegraphSpec,
};
use bevy::prelude::*;
use damage::Health;

const COLUMN: AttackId = AttackId("column");
const WAVE: AttackId = AttackId("wave");

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Maybraid Attack Placeholders".into(),
				resolution: (1280, 720).into(),
				..default()
			}),
			..default()
		}))
		.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.1)))
		.add_plugins(AttacksPlugin)
		.add_plugins(AttackPlaceholderPresentationPlugin)
		.add_systems(Startup, setup)
		.add_systems(Update, retrigger)
		.run();
}

fn setup(
	mut commands: Commands,
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
				shape: HazardShape::Sphere { radius: 1.6 },
				origin: HazardOrigin::AimPoint,
				persist_into_execute: false,
			})
			.authored_hazard(
				HazardShape::Cylinder { radius: 1.6, height: 7.0 },
				HazardOrigin::AimPoint,
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
		Transform::from_xyz(8.0, 10.0, 14.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
	));
	commands.spawn((
		DirectionalLight { illuminance: 6_000.0, ..default() },
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
			PreviewCaster { next: COLUMN, wait: 0.4 },
			Mesh3d(meshes.add(Cuboid::new(1.2, 2.4, 1.2))),
			MeshMaterial3d(materials.add(Color::srgb(0.35, 0.4, 0.48))),
			Transform::from_xyz(0.0, 1.2, 4.0),
		))
		.id();
	let target = commands
		.spawn((
			Name::new("target"),
			Health::from_max(400.0),
			Mesh3d(meshes.add(Capsule3d::new(0.4, 1.2))),
			MeshMaterial3d(materials.add(Color::srgb(0.55, 0.72, 0.84))),
			Transform::from_xyz(0.0, 1.0, -4.0),
		))
		.id();
	commands.entity(owner).insert(PreviewTarget(target));
}

#[derive(Component)]
struct PreviewCaster {
	next: AttackId,
	wait: f32,
}

#[derive(Component)]
struct PreviewTarget(Entity);

fn retrigger(
	time: Res<Time>,
	mut casters: Query<(Entity, &mut PreviewCaster, &PreviewTarget, &AttackUser)>,
	mut begins: MessageWriter<BeginAttack>,
) {
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
