//! Deferred [`MaterialRef`] assignments per column part.

use bevy::prelude::Color;
use material_ref::MaterialRef;

/// Material recipes for pedestal, shaft, and capital.
///
/// Shaft uses the Doric column POM recipe from [#1132](https://github.com/ramate-io/maybraid/issues/1132)
/// once registered in a domain material lib; until then playgrounds fulfill via
/// [`material_ref::StandardMaterialLib`].
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
			shaft: MaterialRef::named("doric-column-shaft-pom")
				.with_palette([Color::srgb(0.78, 0.76, 0.72)]),
			capital: MaterialRef::named("doric-column-capital")
				.with_palette([Color::srgb(0.74, 0.72, 0.68)]),
		}
	}
}
