//! Invisible low-poly icosphere casters for canopy shadows.
//!
//! Bevy 0.19.1 has no shadow-only mesh flag. The clean path is [`RenderLayers`]:
//! proxies live on [`SHADOW_PROXY_RENDER_LAYER`] only, so the main camera
//! (default layer 0) never draws them. Shadow-casting lights gain that layer
//! and still see the icospheres in cascade views. A shared unit ico mesh plus
//! one material instances across plants so the extra casters do not add a
//! draw call per tree.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::scene::LodSceneLevel;

use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::node::FoliageNode;
use crate::procedural::VegetationProceduralAssets;
use crate::scene_children::{pose, scene_children};
use crate::structural_lod::StructuralLod;
use crate::VegetationComponents;

/// Layer reserved for canopy shadow proxies. Isolated capture cameras use 31.
pub const SHADOW_PROXY_RENDER_LAYER: usize = 30;

/// `MAYBRAID_CANOPY_SHADOWS` overlay: proxy (default), full leaf casters, or none.
pub const ENV_CANOPY_SHADOWS: &str = "MAYBRAID_CANOPY_SHADOWS";

/// How canopy participates in sun shadow maps.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanopyShadowMode {
	/// Real canopy stays [`bevy::light::NotShadowCaster`]; icosphere proxies cast.
	#[default]
	Proxy,
	/// Leaf / frond / cheap-ball canopy meshes cast. Proxies are not realized.
	Full,
	/// No canopy casters and no realized proxies (current main + shadows).
	Off,
}

impl CanopyShadowMode {
	pub fn parse_env(raw: &str) -> Option<Self> {
		let raw = raw.trim();
		if raw.is_empty() {
			return None;
		}
		match raw.to_ascii_lowercase().as_str() {
			"proxy" => Some(Self::Proxy),
			"full" => Some(Self::Full),
			"off" => Some(Self::Off),
			other => {
				eprintln!("[{ENV_CANOPY_SHADOWS}] unknown {other:?} (use proxy|full|off)");
				None
			}
		}
	}

	pub fn from_env() -> Self {
		std::env::var(ENV_CANOPY_SHADOWS)
			.ok()
			.as_deref()
			.and_then(Self::parse_env)
			.unwrap_or(Self::Proxy)
	}

	pub fn emit_proxies(self) -> bool {
		matches!(self, Self::Proxy)
	}

	pub fn canopy_casts(self) -> bool {
		matches!(self, Self::Full)
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Proxy => "proxy",
			Self::Full => "full",
			Self::Off => "off",
		}
	}
}

/// Marker realized into a shadow-only unit icosphere.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CanopyShadowProxy;

/// Inserted on a directional once it includes [`SHADOW_PROXY_RENDER_LAYER`].
#[derive(Component, Clone, Copy, Debug)]
struct CanopyShadowSunLayers;

/// Layers the main camera must not share, and the sun must include.
pub fn proxy_render_layers() -> RenderLayers {
	RenderLayers::layer(SHADOW_PROXY_RENDER_LAYER)
}

/// One invisible icosphere at `transform` (unit ico × scale).
pub fn posed_shadow_proxy(transform: Transform) -> impl Scene + 'static {
	bsn! {
		CanopyShadowProxy
		template_value(transform)
		Visibility::Inherited
	}
}

/// One proxy from a plant's [`VegetationComponents::structural_lod`].
pub fn structural_shadow_proxy_scene(
	vegetation: &impl VegetationComponents,
) -> Option<Box<dyn Scene>> {
	let lod = vegetation.structural_lod()?;
	Some(Box::new(posed_shadow_proxy(transform_from_structural(lod))))
}

/// Proxies that match Low / UltraLow cheap-ball canopy sites (one per ball).
pub fn shadow_proxy_scenes_from_foliage(
	nodes: &[FoliageNode],
	level: LodSceneLevel,
) -> Vec<Box<dyn Scene>> {
	let mut scenes = Vec::new();
	for node in nodes {
		match &node.geometry {
			FoliageGeometry::CheapBall => {
				scenes.push(Box::new(posed_shadow_proxy(pose(node.placement))) as Box<dyn Scene>);
			}
			FoliageGeometry::CheapBallCollection(collection) => {
				for placement in collection.placements_for_level(level) {
					scenes.push(Box::new(posed_shadow_proxy(pose(
						node.placement.compose_child(placement),
					))) as Box<dyn Scene>);
				}
			}
			_ => {}
		}
	}
	scenes
}

/// Keep `base` as-is, or parent it with extra shadow-proxy children.
pub fn with_shadow_proxy_children(
	base: impl Scene + 'static,
	proxies: Vec<Box<dyn Scene>>,
) -> impl Scene + 'static {
	if proxies.is_empty() {
		return scene_children(vec![Box::new(base)]);
	}
	let mut children: Vec<Box<dyn Scene>> = vec![Box::new(base)];
	children.extend(proxies);
	scene_children(children)
}

fn transform_from_structural(lod: StructuralLod) -> Transform {
	let radius = lod.tree_radius.max(0.25);
	Transform::from_translation(lod.center).with_scale(Vec3::splat(radius))
}

/// Env mode, realize instanced ico meshes, and put the proxy layer on shadow lights.
pub struct CanopyShadowProxyPlugin;

impl Plugin for CanopyShadowProxyPlugin {
	fn build(&self, app: &mut App) {
		let mode = CanopyShadowMode::from_env();
		app.insert_resource(mode)
			.add_systems(Startup, log_canopy_shadow_mode)
			.add_systems(
				PostUpdate,
				(include_proxy_layer_on_shadow_lights, realize_canopy_shadow_proxies).chain(),
			);
	}
}

fn log_canopy_shadow_mode(mode: Res<CanopyShadowMode>) {
	eprintln!("[{ENV_CANOPY_SHADOWS}] {}", mode.label());
}

fn include_proxy_layer_on_shadow_lights(
	mut commands: Commands,
	lights: Query<
		(Entity, &DirectionalLight, Option<&RenderLayers>),
		Without<CanopyShadowSunLayers>,
	>,
) {
	for (entity, light, existing) in &lights {
		if !light.shadow_maps_enabled {
			continue;
		}
		let layers = existing.cloned().unwrap_or_default().with(SHADOW_PROXY_RENDER_LAYER);
		commands.entity(entity).insert((layers, CanopyShadowSunLayers));
	}
}

fn realize_canopy_shadow_proxies(
	mut commands: Commands,
	added: Query<Entity, (Added<CanopyShadowProxy>, Without<Mesh3d>)>,
	mode: Res<CanopyShadowMode>,
) {
	if !mode.emit_proxies() {
		for entity in &added {
			commands.entity(entity).despawn();
		}
		return;
	}
	let Some(assets) = VegetationProceduralAssets::try_get() else {
		return;
	};
	for entity in &added {
		commands.entity(entity).insert((
			Mesh3d(assets.foliage_ball.clone()),
			MeshMaterial3d(assets.foliage_material.clone()),
			proxy_render_layers(),
		));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::placed::Placement;
	use crate::FoliageNode;

	#[test]
	fn parse_env_accepts_known_tokens() {
		assert_eq!(CanopyShadowMode::parse_env("proxy"), Some(CanopyShadowMode::Proxy));
		assert_eq!(CanopyShadowMode::parse_env("FULL"), Some(CanopyShadowMode::Full));
		assert_eq!(CanopyShadowMode::parse_env("off"), Some(CanopyShadowMode::Off));
		assert_eq!(CanopyShadowMode::parse_env(""), None);
		assert_eq!(CanopyShadowMode::parse_env("nope"), None);
		assert_eq!(ENV_CANOPY_SHADOWS, "MAYBRAID_CANOPY_SHADOWS");
		assert_eq!(SHADOW_PROXY_RENDER_LAYER, 30);
	}

	#[test]
	fn proxy_mode_emits_and_does_not_let_canopy_cast() {
		assert!(CanopyShadowMode::Proxy.emit_proxies());
		assert!(!CanopyShadowMode::Proxy.canopy_casts());
		assert!(!CanopyShadowMode::Full.emit_proxies());
		assert!(CanopyShadowMode::Full.canopy_casts());
		assert!(!CanopyShadowMode::Off.emit_proxies());
		assert!(!CanopyShadowMode::Off.canopy_casts());
	}

	#[test]
	fn structural_proxy_follows_lod_when_present() {
		struct LodPlant(Option<StructuralLod>);
		impl VegetationComponents for LodPlant {
			fn structural_lod(&self) -> Option<StructuralLod> {
				self.0
			}
		}
		assert!(structural_shadow_proxy_scene(&LodPlant(Some(StructuralLod::new(
			Vec3::Y * 8.0,
			4.0
		))))
		.is_some());
		assert!(structural_shadow_proxy_scene(&LodPlant(None)).is_none());
	}

	#[test]
	fn foliage_proxies_one_per_cheap_ball() {
		let nodes = vec![
			FoliageNode::cheap_ball(
				Placement::new(Vec3::Y * 4.0, 0.0).with_scale(Vec3::splat(2.0)),
			),
			FoliageNode::cheap_ball(
				Placement::new(Vec3::Y * 6.0, 0.0).with_scale(Vec3::splat(3.0)),
			),
		];
		assert_eq!(shadow_proxy_scenes_from_foliage(&nodes, LodSceneLevel::Low).len(), 2);
	}

	#[test]
	fn sun_layers_include_default_and_proxy() {
		let layers = RenderLayers::default().with(SHADOW_PROXY_RENDER_LAYER);
		assert!(layers.intersects(&RenderLayers::layer(0)));
		assert!(layers.intersects(&proxy_render_layers()));
		assert!(!proxy_render_layers().intersects(&RenderLayers::layer(0)));
	}

	fn proxy_test_app(mode: CanopyShadowMode) -> App {
		use crate::VegetationProceduralPlugin;

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.add_plugins(VegetationProceduralPlugin)
			.insert_resource(mode);
		app
	}

	#[test]
	fn realize_instances_shared_ico_on_proxy_layer() -> anyhow::Result<()> {
		let mut app = proxy_test_app(CanopyShadowMode::Proxy);
		app.update();
		app.update();

		let entity = app
			.world_mut()
			.spawn((CanopyShadowProxy, Transform::from_scale(Vec3::splat(3.0))))
			.id();
		app.update();

		let world = app.world();
		assert!(world.get::<Mesh3d>(entity).is_some(), "proxy should receive the shared ico");
		assert!(world.get::<MeshMaterial3d<StandardMaterial>>(entity).is_some());
		let layers = world
			.get::<RenderLayers>(entity)
			.ok_or_else(|| anyhow::anyhow!("missing RenderLayers"))?;
		assert!(layers.intersects(&proxy_render_layers()));
		assert!(!layers.intersects(&RenderLayers::layer(0)));
		assert!(world.get::<bevy::light::NotShadowCaster>(entity).is_none());
		Ok(())
	}

	#[test]
	fn shadow_lights_gain_the_proxy_layer() -> anyhow::Result<()> {
		let mut app = proxy_test_app(CanopyShadowMode::Proxy);
		app.update();
		let sun = app
			.world_mut()
			.spawn(DirectionalLight { shadow_maps_enabled: true, ..default() })
			.id();
		let fill = app
			.world_mut()
			.spawn(DirectionalLight { shadow_maps_enabled: false, ..default() })
			.id();
		app.update();
		let sun_layers = app
			.world()
			.get::<RenderLayers>(sun)
			.ok_or_else(|| anyhow::anyhow!("sun should include the proxy layer"))?;
		assert!(sun_layers.intersects(&proxy_render_layers()));
		assert!(sun_layers.intersects(&RenderLayers::layer(0)));
		assert!(app.world().get::<RenderLayers>(fill).is_none());
		Ok(())
	}

	#[test]
	fn full_mode_despawns_unrealized_proxies() -> anyhow::Result<()> {
		let mut app = proxy_test_app(CanopyShadowMode::Full);
		app.update();
		let entity = app.world_mut().spawn((CanopyShadowProxy, Transform::default())).id();
		app.update();
		anyhow::ensure!(app.world().get_entity(entity).is_err(), "full mode must drop proxies");
		Ok(())
	}
}
