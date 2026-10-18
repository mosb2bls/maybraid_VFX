//! Optional cell HUD for Terrain / jersey / Watershed inspection.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham::{
	cascade_chunk_for_cell, PlateauLowPassControllerLayout, Terrain, TerrainCellLayout,
	WatershedLeafKind,
};
use lod::gen::Id;
use lod::hcsg::HcsgStorage;
use std::fmt::{Display, Formatter, Result as FmtResult};

/// Playground debug overlays (cell HUD).
#[derive(Resource, Debug, Clone, Copy)]
pub struct PlaygroundDebugOverlay {
	pub show_cell_hud: bool,
	pub log_cell_changes: bool,
}

impl Default for PlaygroundDebugOverlay {
	fn default() -> Self {
		Self { show_cell_hud: false, log_cell_changes: false }
	}
}

#[derive(Component)]
pub(crate) struct CellLocationHudText;

#[derive(Component)]
pub(crate) struct CellLocationHudRoot;

#[derive(Resource, Default)]
pub(crate) struct LastLoggedCellLocation {
	terrain: Option<(i32, i32)>,
	controller: Option<(i32, i32)>,
	modulation_count: Option<usize>,
	terrain_present: Option<bool>,
}

/// Spawn a small top-of-screen panel for camera cell / stamp inspection (hidden by default).
pub fn setup_cell_location_hud(mut commands: Commands) {
	commands.insert_resource(LastLoggedCellLocation::default());
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(8.0),
				left: Val::Px(8.0),
				right: Val::Px(8.0),
				padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(2.0),
				..default()
			},
			BackgroundColor(Color::srgba(0.05, 0.08, 0.12, 0.82)),
			Visibility::Hidden,
			CellLocationHudRoot,
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new("cell HUD"),
				TextFont { font_size: bevy::text::FontSize::Px(12.0), ..default() },
				TextColor(Color::srgb(0.92, 0.96, 1.0)),
				CellLocationHudText,
			));
		});
}

/// Update the top HUD with terrain / jersey cell under the camera.
pub fn update_cell_location_hud(
	overlay: Res<PlaygroundDebugOverlay>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	layout: Res<TerrainCellLayout>,
	terrain_store: Res<HcsgStorage>,
	mut hud_root: Query<&mut Visibility, With<CellLocationHudRoot>>,
	mut hud: Query<&mut Text, With<CellLocationHudText>>,
	mut last: ResMut<LastLoggedCellLocation>,
) {
	if let Ok(mut visibility) = hud_root.single_mut() {
		*visibility = if overlay.show_cell_hud { Visibility::Visible } else { Visibility::Hidden };
	}

	if !overlay.show_cell_hud && !overlay.log_cell_changes {
		return;
	}

	let Ok(camera) = cameras.single() else {
		return;
	};
	let Ok(plateau_layout) =
		terrain_store.try_entry::<PlateauLowPassControllerLayout>(Id::Universal)
	else {
		return;
	};
	let plateau_layout = plateau_layout.map(|entry| (*entry.value).clone()).unwrap_or_default();

	let p = camera.translation();
	let (tix, tiz) = terrain_cell_coords(&layout, p);
	let (cix, ciz) = controller_cell_coords(&plateau_layout, p);

	let t_size = layout.cell_size.max(1e-3);
	let t_cell = terrain_cell_aabb(tix, tiz, t_size, layout.vertical_half_extent);
	let c_cell = plateau_layout.cell_bounds(cix, ciz);

	let Ok(overlapping) = terrain_store.try_overlapping::<Terrain>(t_cell) else {
		return;
	};
	let terrain = overlapping
		.into_iter()
		.filter_map(|id| terrain_store.try_entry::<Terrain>(id).ok().flatten())
		.map(|entry| entry.value)
		.find(|terrain| cells_match_xz(&terrain.cell, &t_cell));
	let terrain = terrain.as_deref();
	let leaf_under_cam =
		terrain.and_then(|t| t.jersey_leaves.iter().find(|leaf| point_in_xz(p, leaf)).copied());
	let marazion_under_cam = terrain
		.and_then(|t| t.marazion_leaves.iter().find(|leaf| point_in_xz(p, &leaf.cell)).copied());

	let report = CellLocationReport {
		cam: p,
		terrain_ix: tix,
		terrain_iz: tiz,
		terrain_layout_size: t_size,
		terrain_cell: t_cell,
		terrain: terrain.map(TerrainReport::from_terrain),
		controller_ix: cix,
		controller_iz: ciz,
		controller_layout_size: plateau_layout.grid.cell_size,
		controller_origin_offset: plateau_layout.grid.origin_offset,
		controller_cell: c_cell,
		jersey_leaf: leaf_under_cam,
		marazion_leaf: marazion_under_cam,
	};

	let rendered = report.to_string();
	if overlay.show_cell_hud {
		if let Ok(mut text) = hud.single_mut() {
			*text = Text::new(rendered.clone());
		}
	}

	if !overlay.log_cell_changes {
		return;
	}

	let terrain_present = report.terrain.is_some();
	let modulation_count = report.terrain.as_ref().map(|t| t.ops).unwrap_or(0);
	let changed = last.terrain != Some((tix, tiz))
		|| last.controller != Some((cix, ciz))
		|| last.terrain_present != Some(terrain_present)
		|| last.modulation_count != Some(modulation_count);
	if changed {
		last.terrain = Some((tix, tiz));
		last.controller = Some((cix, ciz));
		last.terrain_present = Some(terrain_present);
		last.modulation_count = Some(modulation_count);
		info!("\n{rendered}");
	}
}

#[derive(Debug, Clone)]
struct CellLocationReport {
	cam: Vec3,
	terrain_ix: i32,
	terrain_iz: i32,
	terrain_layout_size: f32,
	terrain_cell: Aabb3d,
	terrain: Option<TerrainReport>,
	controller_ix: i32,
	controller_iz: i32,
	controller_layout_size: f32,
	controller_origin_offset: Vec2,
	controller_cell: Aabb3d,
	jersey_leaf: Option<Aabb3d>,
	marazion_leaf: Option<durham::WatershedLeafBounds>,
}

#[derive(Debug, Clone)]
struct TerrainReport {
	cell: Aabb3d,
	chunk: Aabb3d,
	res_2: u8,
	ops: usize,
	jersey_leaves: usize,
	marazion_leaves: usize,
	marazion_lake: usize,
	marazion_stream: usize,
	marazion_streams_graph: usize,
	marazion_bog: usize,
	marazion_empty: usize,
}

impl TerrainReport {
	fn from_terrain(t: &Terrain) -> Self {
		let chunk = cascade_chunk_for_cell(t.cell, t.res_2);
		let extent = chunk.extent_vec();
		let mut marazion_lake = 0usize;
		let mut marazion_stream = 0usize;
		let mut marazion_streams_graph = 0usize;
		let mut marazion_bog = 0usize;
		let mut marazion_empty = 0usize;
		for leaf in &t.marazion_leaves {
			match leaf.kind {
				WatershedLeafKind::Lake => marazion_lake += 1,
				WatershedLeafKind::Stream => marazion_stream += 1,
				WatershedLeafKind::StreamsGraph => marazion_streams_graph += 1,
				WatershedLeafKind::Bog => marazion_bog += 1,
				WatershedLeafKind::Empty => marazion_empty += 1,
			}
		}
		Self {
			cell: t.cell,
			chunk: Aabb3d::from_min_max(chunk.origin, chunk.origin + extent),
			res_2: t.res_2,
			ops: t.modulations.len(),
			jersey_leaves: t.jersey_leaves.len(),
			marazion_leaves: t.marazion_leaves.len(),
			marazion_lake,
			marazion_stream,
			marazion_streams_graph,
			marazion_bog,
			marazion_empty,
		}
	}
}

impl Display for CellLocationReport {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		writeln!(f, "── camera cell report ──────────────────────────────")?;
		writeln!(f, "cam          {:?}", (self.cam.x, self.cam.y, self.cam.z))?;
		writeln!(f)?;
		writeln!(
			f,
			"terrain      T({}, {})  layout_size={:.3}",
			self.terrain_ix, self.terrain_iz, self.terrain_layout_size
		)?;
		writeln!(f, "  cell AABB  {}", fmt_aabb(&self.terrain_cell))?;
		match &self.terrain {
			Some(t) => {
				writeln!(f, "  status     GENERATED  res_2={}", t.res_2)?;
				writeln!(f, "  cell AABB  {}", fmt_aabb(&t.cell))?;
				writeln!(f, "  chunk AABB {}", fmt_aabb(&t.chunk))?;
				writeln!(
					f,
					"  modulations n={}  jersey_leaves n={}  marazion_leaves n={}",
					t.ops, t.jersey_leaves, t.marazion_leaves
				)?;
				writeln!(
					f,
					"  marazion    lake={} stream={} streams_graph={} bog={} empty={}",
					t.marazion_lake,
					t.marazion_stream,
					t.marazion_streams_graph,
					t.marazion_bog,
					t.marazion_empty
				)?;
			}
			None => writeln!(f, "  status     NOT GENERATED")?,
		}
		writeln!(f)?;
		writeln!(
			f,
			"plateau ctl  C({}, {})  layout_size={:.3}  origin_offset={:?}",
			self.controller_ix,
			self.controller_iz,
			self.controller_layout_size,
			(self.controller_origin_offset.x, self.controller_origin_offset.y)
		)?;
		writeln!(f, "  cell AABB  {}", fmt_aabb(&self.controller_cell))?;
		match self.jersey_leaf {
			Some(leaf) => {
				writeln!(f, "jersey leaf  UNDER CAMERA")?;
				writeln!(f, "  leaf AABB  {}", fmt_aabb(&leaf))?;
			}
			None => writeln!(f, "jersey leaf  (none under camera)")?,
		}
		match self.marazion_leaf {
			Some(leaf) => {
				writeln!(
					f,
					"marazion     UNDER CAMERA  kind={}  band={}",
					leaf.kind.label(),
					leaf.band.label()
				)?;
				writeln!(f, "  leaf AABB  {}", fmt_aabb(&leaf.cell))?;
			}
			None => writeln!(f, "marazion     (none under camera)")?,
		}
		write!(f, "───────────────────────────────────────────────────")
	}
}

fn fmt_aabb(a: &Aabb3d) -> String {
	format!(
		"min=({:.3}, {:.3}, {:.3})  max=({:.3}, {:.3}, {:.3})",
		a.min.x, a.min.y, a.min.z, a.max.x, a.max.y, a.max.z
	)
}

fn terrain_cell_aabb(ix: i32, iz: i32, cell_size: f32, vertical_half_extent: f32) -> Aabb3d {
	let size = cell_size.max(1e-3);
	let vy = vertical_half_extent.max(size);
	Aabb3d::from_min_max(
		Vec3::new(ix as f32 * size, -vy, iz as f32 * size),
		Vec3::new((ix + 1) as f32 * size, vy, (iz + 1) as f32 * size),
	)
}

fn terrain_cell_coords(layout: &TerrainCellLayout, p: Vec3) -> (i32, i32) {
	let s = layout.cell_size.max(1e-3);
	((p.x / s).floor() as i32, (p.z / s).floor() as i32)
}

fn controller_cell_coords(layout: &PlateauLowPassControllerLayout, p: Vec3) -> (i32, i32) {
	let s = layout.grid.cell_size.max(1e-3);
	let gx = p.x - layout.grid.origin_offset.x;
	let gz = p.z - layout.grid.origin_offset.y;
	((gx / s).floor() as i32, (gz / s).floor() as i32)
}

fn cells_match_xz(a: &Aabb3d, b: &Aabb3d) -> bool {
	(a.min.x - b.min.x).abs() < 1e-3
		&& (a.min.z - b.min.z).abs() < 1e-3
		&& (a.max.x - b.max.x).abs() < 1e-3
		&& (a.max.z - b.max.z).abs() < 1e-3
}

fn point_in_xz(p: Vec3, cell: &Aabb3d) -> bool {
	p.x >= cell.min.x && p.x < cell.max.x && p.z >= cell.min.z && p.z < cell.max.z
}
