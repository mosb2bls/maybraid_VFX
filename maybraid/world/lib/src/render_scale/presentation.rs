//! Presentation materials for visual style upscale treatments.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use super::visual_style::PresentationTreatment;

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
		concat!(
			"embedded://",
			env!("CARGO_CRATE_NAME"),
			"/",
			"presentation.wgsl#impressionist_fragment"
		)
		.into()
	}

	fn alpha_mode(&self) -> AlphaMode2d {
		AlphaMode2d::Opaque
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
		concat!(
			"embedded://",
			env!("CARGO_CRATE_NAME"),
			"/",
			"presentation.wgsl#pre_post_modern_fragment"
		)
		.into()
	}

	fn alpha_mode(&self) -> AlphaMode2d {
		AlphaMode2d::Opaque
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
		concat!(
			"embedded://",
			env!("CARGO_CRATE_NAME"),
			"/",
			"presentation.wgsl#nearest_neighbor_fragment"
		)
		.into()
	}

	fn alpha_mode(&self) -> AlphaMode2d {
		AlphaMode2d::Opaque
	}
}

/// Handle to the active presentation material, typed by treatment.
#[derive(Clone, Debug)]
pub enum PresentationMaterial {
	Impressionist(Handle<ImpressionistMaterial>),
	PrePostModern(Handle<PrePostModernMaterial>),
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
	_nearest_neighbor: &mut Assets<NearestNeighborMaterial>,
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
		PresentationTreatment::NearestNeighbor | PresentationTreatment::None => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

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
		assert!(
			create_presentation_material(
				PresentationTreatment::NearestNeighbor,
				Handle::default(),
				UVec2::new(540, 304),
				UVec2::new(1920, 1080),
				&mut impressionist,
				&mut pre_post_modern,
				&mut nearest_neighbor,
			)
			.is_none(),
			"Retro/Antediluvian use a nearest-sampled sprite, not a custom shader"
		);
	}
}
