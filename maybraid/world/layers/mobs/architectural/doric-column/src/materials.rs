//! Deferred [`MaterialRef`] assignments per column part.

use bevy::prelude::Color;
use building_shaders::doric_column_kit_shaft_ref;
use material_ref::MaterialRef;

/// Material recipes for pedestal, shaft, and capital.
///
/// Shaft uses the Doric column POM recipe from [#1132](https://github.com/ramate-io/maybraid/issues/1132)
/// via [`RECIPE_DORIC_COLUMN_SHAFT_KIT`] / [`building_shaders::is_column_surface_recipe`].
#[derive(Debug, Clone, PartialEq)]
pub struct DoricColumnMaterials {
	pub pedestal: MaterialRef,
	pub shaft: MaterialRef,
	pub capital: MaterialRef,
}

impl Default for DoricColumnMaterials {
	fn default() -> Self {
		Self {
			pedestal: MaterialRef::named("doric-column-pedestal")
				.with_palette([Color::srgb(0.72, 0.70, 0.66)]),
			shaft: doric_column_kit_shaft_ref(),
			capital: MaterialRef::named("doric-column-capital")
				.with_palette([Color::srgb(0.74, 0.72, 0.68)]),
		}
	}
}

#[cfg(test)]
mod tests {
	use building_shaders::{is_column_surface_recipe, ColumnSurfaceMaterial, DEFAULT_RELIEF_DEPTH_D};
	use material_ref::MaterialRefPlugin;

	use super::*;
	use crate::spec::DoricColumnSpec;

	#[test]
	fn default_shaft_recipe_matches_kit_contract() {
		use material_ref::MaterialId;

		let shaft = DoricColumnMaterials::default().shaft;
		assert_eq!(
			shaft.name,
			MaterialId::named(building_shaders::RECIPE_DORIC_COLUMN_SHAFT_KIT)
		);
		let MaterialId::Name(name) = &shaft.name else {
			panic!("shaft recipe must be named");
		};
		assert!(is_column_surface_recipe(name));
	}

	#[test]
	fn default_spec_shaft_resolves_column_surface_material() {
		use bevy::prelude::*;

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			building_shaders::BuildingShadersPlugin,
			building_shaders::UrbanMaterialRefPlugin,
			MaterialRefPlugin::<building_shaders::UrbanSurfaceMaterialLib<'_>>::default(),
		))
		.init_asset::<StandardMaterial>()
		.init_asset::<building_shaders::UrbanSurfaceMaterial>()
		.init_asset::<ColumnSurfaceMaterial>();

		let spec = DoricColumnSpec::default();
		let entity = app
			.world_mut()
			.spawn(material_ref::MaterialRefRoot(spec.materials.shaft.clone()))
			.id();
		app.update();

		let material = app
			.world()
			.get::<MeshMaterial3d<ColumnSurfaceMaterial>>(entity)
			.expect("shaft fulfills as column surface");
		let handle = &material.0;
		let assets = app.world().resource::<Assets<ColumnSurfaceMaterial>>();
		let resolved = assets.get(handle).expect("material asset");
		assert_eq!(resolved.params.kind, building_shaders::KIND_DORIC);
		// Default relief uses documented D-relative depth when scalar 7 is unset.
		assert!((resolved.params.scalars[1].w - 0.0).abs() < 1e-5);
		let material_ref = &spec.materials.shaft;
		let packed = ColumnSurfaceMaterial::from_material_ref(material_ref);
		assert_eq!(packed.params.scalars[0].y, material_ref.scalar_values()[1]);
		assert_eq!(DEFAULT_RELIEF_DEPTH_D, 0.11);
	}
}
