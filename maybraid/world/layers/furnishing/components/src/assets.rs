//! Runtime paths for furniture GLBs (under `maybraid/assets`).
//!
//! Sources are the Blender kits from [`ecb5a8a`](https://github.com/ramate-io/maybraid/commit/ecb5a8a31d2894311fa690cfbe8ab01cee385fba)
//! in `maybraid/art/furniture/` (skip `.blend1`). Export scene 0 into these
//! paths. Authored space is documented in [`crate::kit_space`].

use building_components::AssetPath;

/// Bed frame kit (`X,Y \in [-1,1]\), \(Z \in [0,1]\)` in Blender).
pub const BEDFRAME_001: AssetPath = AssetPath::new("furniture/bed/bedframe/bedframe_001.glb");
/// Mattress kit (same authored box as the frame).
pub const MATTRESS_001: AssetPath = AssetPath::new("furniture/bed/mattress/mattress_001.glb");
/// Covers kit (same authored box as the mattress — same slot transform).
pub const COVERS_001: AssetPath = AssetPath::new("furniture/bed/covers/covers_001.glb");

/// Chest trunk.
pub const CHEST_TRUNK_001: AssetPath = AssetPath::new("furniture/chest/trunk/trunk_001.glb");
/// Chest lid (sits on the trunk).
pub const CHEST_LID_001: AssetPath = AssetPath::new("furniture/chest/lid/lid_001.glb");
/// Chest latch (Blender \(Y \in [0,1]\) from the front, \(X,Z \in [-1,1]\)).
pub const CHEST_LATCH_001: AssetPath = AssetPath::new("furniture/chest/latch/latch_001.glb");

/// Chair leg tube.
pub const CHAIR_LEG_001: AssetPath = AssetPath::new("furniture/chair/legs/chair_leg_001.glb");
/// Chair seat.
pub const CHAIR_SEAT_001: AssetPath = AssetPath::new("furniture/chair/seat/chair_seat_001.glb");
/// Chair back (authored \(+Y\) → engine \(+Z\)).
pub const CHAIR_BACK_001: AssetPath = AssetPath::new("furniture/chair/back/chair_back_001.glb");

/// Counter footer.
pub const COUNTER_FOOTER_001: AssetPath =
	AssetPath::new("furniture/counter/counter_footer/counter_footer_001.glb");
/// Counter volume.
pub const COUNTER_VOLUME_001: AssetPath =
	AssetPath::new("furniture/counter/counter_volume/counter_volume_001.glb");
/// Countertop (over-sails the volume).
pub const COUNTER_TOP_001: AssetPath =
	AssetPath::new("furniture/counter/countertop/countertop_001.glb");

/// Diner food-display case (sit-on).
pub const FOOD_DISPLAY_001: AssetPath =
	AssetPath::new("furniture/food_display/case/food_display_001.glb");

pub const APPLE_001: AssetPath = AssetPath::new("furniture/fruit/apple/apple_001.glb");
pub const ORANGE_001: AssetPath = AssetPath::new("furniture/fruit/orange/orange_001.glb");
pub const PEAR_001: AssetPath = AssetPath::new("furniture/fruit/pear/pear_001.glb");
pub const BANANA_001: AssetPath = AssetPath::new("furniture/fruit/banana/banana_001.glb");

pub const LOAF_001: AssetPath = AssetPath::new("furniture/bread/loaf/loaf_001.glb");
pub const BAGUETTE_001: AssetPath = AssetPath::new("furniture/bread/baguette/baguette_001.glb");
pub const BUN_001: AssetPath = AssetPath::new("furniture/bread/bun/bun_001.glb");
pub const BOULE_001: AssetPath = AssetPath::new("furniture/bread/boule/boule_001.glb");

pub const SHELF_ROW_001: AssetPath = AssetPath::new("furniture/shelf/row/shelf_row_001.glb");

pub const RANGE_BODY_001: AssetPath = AssetPath::new("furniture/range/body/range_body_001.glb");
pub const RANGE_DOOR_001: AssetPath = AssetPath::new("furniture/range/door/range_door_001.glb");
pub const RANGE_BURNER_001: AssetPath =
	AssetPath::new("furniture/range/burner/range_burner_001.glb");
pub const RANGE_KNOB_001: AssetPath = AssetPath::new("furniture/range/knob/range_knob_001.glb");

pub const BASIN_001: AssetPath = AssetPath::new("furniture/sink/basin/basin_001.glb");
pub const FAUCET_001: AssetPath = AssetPath::new("furniture/sink/faucet/faucet_001.glb");

pub const FRIDGE_BODY_001: AssetPath =
	AssetPath::new("furniture/refrigerator/body/refrigerator_body_001.glb");
pub const FRIDGE_DOOR_001: AssetPath =
	AssetPath::new("furniture/refrigerator/door/refrigerator_door_001.glb");

pub const POT_001: AssetPath = AssetPath::new("furniture/cookware/pot/pot_001.glb");
pub const POT_LID_001: AssetPath = AssetPath::new("furniture/cookware/lid/pot_lid_001.glb");
pub const SKILLET_001: AssetPath = AssetPath::new("furniture/cookware/skillet/skillet_001.glb");
pub const SAUCEPAN_001: AssetPath = AssetPath::new("furniture/cookware/saucepan/saucepan_001.glb");

pub const TABLE_TOP_001: AssetPath = AssetPath::new("furniture/table/top/table_top_001.glb");
pub const TABLE_TOP_ROUND_001: AssetPath =
	AssetPath::new("furniture/table/top/table_top_round_001.glb");
pub const TABLE_LEG_001: AssetPath = AssetPath::new("furniture/table/leg/table_leg_001.glb");
pub const TABLE_APRON_001: AssetPath = AssetPath::new("furniture/table/apron/table_apron_001.glb");
pub const TABLE_PEDESTAL_001: AssetPath =
	AssetPath::new("furniture/table/pedestal/table_pedestal_001.glb");
