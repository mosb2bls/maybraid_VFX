//! Landform stamp families ([RFC-105 §3.8](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain)).

pub mod basin_water;
pub mod canyon;
pub mod cave_network;
pub mod hydrology_complex;
pub mod karst_pocket;
pub mod plateau_cap;
pub mod pocket_water;
pub mod rolling_ground;
pub mod rugged_massif;
pub mod valley_basin;
pub mod valley_train;

pub use basin_water::{BasinWater, BasinWaterParams};
pub use canyon::{Canyon, CanyonParams, CanyonVariant};
pub use cave_network::{CaveNetwork, CaveNetworkParams, CaveSegment, CaveSegmentKind};
pub use hydrology_complex::{HydrologyComplex, HydrologyComplexKind, HydrologyComplexParams};
pub use karst_pocket::{KarstNavClass, KarstPocket, KarstPocketParams};
pub use plateau_cap::{PlateauCap, PlateauCapParams, PlateauFootprint, PlateauSurfaceClass};
pub use pocket_water::{PocketTermination, PocketWater, PocketWaterParams};
pub use rolling_ground::{RollingGround, RollingGroundParams};
pub use rugged_massif::{MassifStyle, RuggedMassif, RuggedMassifParams};
pub use valley_basin::{ValleyBasin, ValleyBasinParams, ValleyCrossSection, ValleyFloorKind};
pub use valley_train::{
	ValleyTrain, ValleyTrainParams, ValleyTrainSegment, ValleyTrainSegmentRole,
};
