//! Capture subject contract and built-in scene descriptions.

use bevy::prelude::{Color, Vec3};
use scene_ref::SceneRef;

use crate::id::CaptureId;
use crate::view::CaptureView;

/// A subject that can be captured in isolation.
///
/// The id names visual appearance and must stay stable across launches.
pub trait CaptureSubject: Send + Sync + Clone + 'static {
	fn capture_id(&self) -> CaptureId;
	fn capture_scene(&self) -> CaptureScene;
	fn preferred_view(&self) -> CaptureView;
}

/// Visual scene supplied by a [`CaptureSubject`].
///
/// Prefer [`CaptureScene::SceneRef`] when the subject is a single GLB. Mesh
/// primitives exist for tests and headless demos that should not load assets.
/// [`CaptureScene::Host`] is an empty isolated root; the application realizes
/// the posed subject and stamps [`crate::CaptureHostReady`] when meshes and
/// materials are in place.
#[derive(Clone, Debug, PartialEq)]
pub enum CaptureScene {
	SceneRef(SceneRef),
	Mesh(CaptureMesh),
	Host { kind: String, fragment: String },
}

/// Procedural mesh used when a GLB is not required.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptureMesh {
	pub kind: CaptureMeshKind,
	pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaptureMeshKind {
	Cuboid,
	Sphere,
}

impl CaptureScene {
	pub fn scene_ref(path: impl Into<String>) -> Self {
		Self::SceneRef(SceneRef::glb(path))
	}

	pub fn cuboid(color: [f32; 4]) -> Self {
		Self::Mesh(CaptureMesh { kind: CaptureMeshKind::Cuboid, color })
	}

	pub fn sphere(color: [f32; 4]) -> Self {
		Self::Mesh(CaptureMesh { kind: CaptureMeshKind::Sphere, color })
	}

	pub fn host(kind: impl Into<String>, fragment: impl Into<String>) -> Self {
		Self::Host { kind: kind.into(), fragment: fragment.into() }
	}

	pub fn cache_fragment(&self) -> String {
		match self {
			Self::SceneRef(scene) => format!("ref:{}", scene.labeled_path()),
			Self::Mesh(mesh) => mesh.cache_fragment(),
			Self::Host { kind, fragment } => format!("host:{kind}:{fragment}"),
		}
	}
}

impl CaptureMesh {
	pub fn cache_fragment(self) -> String {
		format!(
			"mesh:{:?}:{:x}:{:x}:{:x}:{:x}",
			self.kind,
			self.color[0].to_bits(),
			self.color[1].to_bits(),
			self.color[2].to_bits(),
			self.color[3].to_bits(),
		)
	}

	pub fn color(self) -> Color {
		Color::srgba(self.color[0], self.color[1], self.color[2], self.color[3])
	}
}

/// Built-in subject: a colored cuboid framed in 3/4 view.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshCaptureSubject {
	pub id: CaptureId,
	pub mesh: CaptureMesh,
	pub view: CaptureView,
}

impl MeshCaptureSubject {
	pub fn cuboid(color: [f32; 4]) -> Self {
		Self {
			id: CaptureId::new(format!(
				"mesh:cuboid:{:x}-{:x}-{:x}-{:x}",
				color[0].to_bits(),
				color[1].to_bits(),
				color[2].to_bits(),
				color[3].to_bits(),
			)),
			mesh: CaptureMesh { kind: CaptureMeshKind::Cuboid, color },
			view: CaptureView::look_at(Vec3::new(1.4, 1.0, 1.4), Vec3::ZERO),
		}
	}
}

impl CaptureSubject for MeshCaptureSubject {
	fn capture_id(&self) -> CaptureId {
		self.id.clone()
	}

	fn capture_scene(&self) -> CaptureScene {
		CaptureScene::Mesh(self.mesh)
	}

	fn preferred_view(&self) -> CaptureView {
		self.view
	}
}

impl CaptureSubject for SceneRef {
	fn capture_id(&self) -> CaptureId {
		CaptureId::new(self.labeled_path())
	}

	fn capture_scene(&self) -> CaptureScene {
		CaptureScene::SceneRef(self.clone())
	}

	fn preferred_view(&self) -> CaptureView {
		CaptureView::default()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cuboid_variants_have_distinct_ids() -> anyhow::Result<()> {
		let red = MeshCaptureSubject::cuboid([1.0, 0.0, 0.0, 1.0]);
		let blue = MeshCaptureSubject::cuboid([0.0, 0.0, 1.0, 1.0]);
		assert_ne!(red.capture_id(), blue.capture_id());
		assert_eq!(red.capture_id(), red.capture_id());
		Ok(())
	}

	#[test]
	fn scene_ref_reuses_labeled_path() -> anyhow::Result<()> {
		let scene = SceneRef::glb("items/guns/concepts/bullpup_full_concept.glb");
		assert_eq!(
			scene.capture_id().as_str(),
			"items/guns/concepts/bullpup_full_concept.glb#Scene0"
		);
		Ok(())
	}

	#[test]
	fn host_cache_fragment_names_the_application_kind() -> anyhow::Result<()> {
		let scene = CaptureScene::host("generated-firearm", "firearm:body=silopup");
		assert_eq!(scene.cache_fragment(), "host:generated-firearm:firearm:body=silopup");
		Ok(())
	}
}
