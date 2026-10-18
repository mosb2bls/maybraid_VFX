//! Shared world stack types for HCSG furniture hosts.

use durham::Durham;
use maputo::Furnished;
use richmond::Richmond;
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;

/// Furnished cell type for the assembled world stack.
pub type WorldFurnished = Furnished<Urbanization<Richmond<OnTerrain<Durham>>>>;
