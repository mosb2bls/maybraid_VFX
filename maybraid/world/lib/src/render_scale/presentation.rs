//! Presentation materials for visual style upscale treatments.

use std::borrow::Cow;

use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
	AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin};

use super::visual_style::PresentationTreatment;

/// Asset path Bevy registers for `embedded_asset!(app, "presentation.wgsl")`
/// called from this module (`src` is stripped; the subdirectory is not).
///
/// Do not append `#entry` — that is an asset-server label, not a WGSL
/// function. [`Material2d::specialize`] selects the fragment entry point.
const PRESENTATION_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/render_scale/presentation.wgsl");

fn use_presentation_entry(descriptor: &mut RenderPipelineDescriptor, entry: &'static str) {
	if let Some(fragment) = descriptor.fragment.as_mut() {
		fragment.entry_point = Some(Cow::Borrowed(entry));
	}
}

/// Parameters passed to presentation shaders.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TreatmentParams {
	pub source_size: Vec2,
	pub target_size: Vec2,
}

/// Impressionist presentation material: linear upscale with painterly softening.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ImpressionistMaterial {
	#[texture(0)]
	#[sampler(1)]
	pub source: Handle<Image>,
	#[uniform(2)]
	params: TreatmentParams,
}

impl Material2d for ImpressionistMaterial {
	fn fragment_shader() -> ShaderRef {
		PRESENTATION_SHADER.into()
	}

	fn alpha_mode(&self) -> AlphaMode2d {
		AlphaMode2d::Opaque
	}

	fn specialize(
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: Material2dKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		use_presentation_entry(descriptor, "impressionist_fragment");
		Ok(())
	}
}

impl ImpressionistMaterial {
	pub fn new(source: Handle<Image>, source_size: UVec2, target_size: UVec2) -> Self {
		Self {
			source,
			params: TreatmentParams {
				source_size: source_size.as_vec2(),
				target_size: target_size.as_vec2(),
			},
		}
	}
}

/// Pre-post-modern presentation material: misregistered color-print effect.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PrePostModernMaterial {
	#[texture(0)]
	#[sampler(1)]
	pub source: Handle<Image>,
	#[uniform(2)]
	params: TreatmentParams,
}

impl Material2d for PrePostModernMaterial {
	fn fragment_shader() -> ShaderRef {
		PRESENTATION_SHADER.into()
	}

	fn alpha_mode(&self) -> AlphaMode2d {
		AlphaMode2d::Opaque
	}

	fn specialize(
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: Material2dKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		use_presentation_entry(descriptor, "pre_post_modern_fragment");
		Ok(())
	}
}

impl PrePostModernMaterial {
	pub fn new(source: Handle<Image>, source_size: UVec2, target_size: UVec2) -> Self {
		Self {
			source,
			params: TreatmentParams {
				source_size: source_size.as_vec2(),
				target_size: target_size.as_vec2(),
			},
		}
	}
}

/// Nearest-neighbor presentation material: single nearest sample, no filtering.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct NearestNeighborMaterial {
	#[texture(0)]
	#[sampler(1)]
	pub source: Handle<Image>,
	#[uniform(2)]
	params: TreatmentParams,
}

impl Material2d for NearestNeighborMaterial {
	fn fragment_shader() -> ShaderRef {
		PRESENTATION_SHADER.into()
	}

	fn alpha_mode(&self) -> AlphaMode2d {
		AlphaMode2d::Opaque
	}

	fn specialize(
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: Material2dKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		use_presentation_entry(descriptor, "nearest_neighbor_fragment");
		Ok(())
	}
}

impl NearestNeighborMaterial {
	pub fn new(source: Handle<Image>, source_size: UVec2, target_size: UVec2) -> Self {
		Self {
			source,
			params: TreatmentParams {
				source_size: source_size.as_vec2(),
				target_size: target_size.as_vec2(),
			},
		}
	}
}

/// Handle to the active presentation material, typed by treatment.
#[derive(Clone, Debug)]
pub enum PresentationMaterial {
	Impressionist(Handle<ImpressionistMaterial>),
	PrePostModern(Handle<PrePostModernMaterial>),
	NearestNeighbor(Handle<NearestNeighborMaterial>),
}

pub struct PresentationMaterialsPlugin;

impl Plugin for PresentationMaterialsPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "presentation.wgsl");
		app.add_plugins((
			Material2dPlugin::<ImpressionistMaterial>::default(),
			Material2dPlugin::<PrePostModernMaterial>::default(),
			Material2dPlugin::<NearestNeighborMaterial>::default(),
		));
	}
}

pub fn create_presentation_material(
	treatment: PresentationTreatment,
	source: Handle<Image>,
	source_size: UVec2,
	target_size: UVec2,
	impressionist: &mut Assets<ImpressionistMaterial>,
	pre_post_modern: &mut Assets<PrePostModernMaterial>,
	nearest_neighbor: &mut Assets<NearestNeighborMaterial>,
) -> Option<PresentationMaterial> {
	match treatment {
		PresentationTreatment::Impressionist => {
			let material = ImpressionistMaterial::new(source, source_size, target_size);
			Some(PresentationMaterial::Impressionist(impressionist.add(material)))
		}
		PresentationTreatment::PrePostModern => {
			let material = PrePostModernMaterial::new(source, source_size, target_size);
			Some(PresentationMaterial::PrePostModern(pre_post_modern.add(material)))
		}
		PresentationTreatment::NearestNeighbor => {
			let material = NearestNeighborMaterial::new(source, source_size, target_size);
			Some(PresentationMaterial::NearestNeighbor(nearest_neighbor.add(material)))
		}
		PresentationTreatment::None => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::embedded_path;

	#[test]
	fn shader_ref_matches_embedded_asset_path() {
		let path = embedded_path!("presentation.wgsl");
		let registered = path.to_str().unwrap().replace('\\', "/");
		assert_eq!(
			registered,
			format!("{}/render_scale/presentation.wgsl", env!("CARGO_CRATE_NAME")),
			"Material2d ShaderRef must use this path; Bevy embeds subdirectory WGSL under src/"
		);
		assert!(PRESENTATION_SHADER.ends_with(&registered));
		assert!(
			!PRESENTATION_SHADER.contains('#'),
			"ShaderRef #suffix is an asset label; WGSL entries are selected in specialize"
		);
	}

	#[test]
	fn none_treatment_has_no_material() {
		let mut impressionist = Assets::<ImpressionistMaterial>::default();
		let mut pre_post_modern = Assets::<PrePostModernMaterial>::default();
		let mut nearest_neighbor = Assets::<NearestNeighborMaterial>::default();
		assert!(create_presentation_material(
			PresentationTreatment::None,
			Handle::default(),
			UVec2::new(700, 394),
			UVec2::new(1920, 1080),
			&mut impressionist,
			&mut pre_post_modern,
			&mut nearest_neighbor,
		)
		.is_none());
	}

	#[test]
	fn each_treatment_adds_a_typed_material() {
		let mut impressionist = Assets::<ImpressionistMaterial>::default();
		let mut pre_post_modern = Assets::<PrePostModernMaterial>::default();
		let mut nearest_neighbor = Assets::<NearestNeighborMaterial>::default();
		assert!(matches!(
			create_presentation_material(
				PresentationTreatment::Impressionist,
				Handle::default(),
				UVec2::new(700, 394),
				UVec2::new(1920, 1080),
				&mut impressionist,
				&mut pre_post_modern,
				&mut nearest_neighbor,
			),
			Some(PresentationMaterial::Impressionist(_))
		));
		assert!(matches!(
			create_presentation_material(
				PresentationTreatment::PrePostModern,
				Handle::default(),
				UVec2::new(1280, 720),
				UVec2::new(1920, 1080),
				&mut impressionist,
				&mut pre_post_modern,
				&mut nearest_neighbor,
			),
			Some(PresentationMaterial::PrePostModern(_))
		));
		assert!(matches!(
			create_presentation_material(
				PresentationTreatment::NearestNeighbor,
				Handle::default(),
				UVec2::new(540, 304),
				UVec2::new(1920, 1080),
				&mut impressionist,
				&mut pre_post_modern,
				&mut nearest_neighbor,
			),
			Some(PresentationMaterial::NearestNeighbor(_))
		));
	}
}
