use bevy_math::Vec3;
use character_items::ClothingKind;
use character_ui_menu::{CameraFocus, FocusRig};
use characters::species::{
	chupri::pose::CHUPRI_OVERALL_SCALE, kappler::pose::KAPPLER_OVERALL_SCALE,
	topple::pose::TOPPLE_OVERALL_SCALE,
};

pub const BODY_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-1.0, 1.0, 4.0), Vec3::new(2.0, 0.0, -2.0));

/// Default front-on body framing (full-length garments in the tile).
pub const SPIN_REVEAL_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(0.0, 1.0, 4.2), Vec3::new(0.0, 1.0, 0.0));

/// World-space framing for an unskinned garment. Fitted meshes are hip-origin
/// (waist at y = 0, legs in −Y), so lowers look below the origin.
pub fn spin_reveal_focus(kind: ClothingKind) -> CameraFocus {
	let (camera, look) = match kind {
		ClothingKind::Lower => (Vec3::new(0.0, -0.5, 3.0), Vec3::new(0.0, -0.5, 0.0)),
		ClothingKind::Upper => (Vec3::new(0.0, 0.35, 3.0), Vec3::new(0.0, 0.35, 0.0)),
		ClothingKind::Footwear => (Vec3::new(0.0, -0.85, 2.4), Vec3::new(0.0, -0.85, 0.0)),
		ClothingKind::Full => (Vec3::new(0.0, -0.15, 4.2), Vec3::new(0.0, -0.15, 0.0)),
	};
	CameraFocus::new(FocusRig::Body, "root", camera, look)
}

/// World-space framing for an assembled firearm kit at the origin.
///
/// Menu inspect prefers FOV × kit-AABB distance from a side 3/4 aimed at the
/// receiver so the barrel stays in frame and the kit sits in the pane.
pub fn spin_reveal_firearm_focus() -> CameraFocus {
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-3.6, 1.6, 0.8), Vec3::new(0.0, 0.2, 0.0))
}

/// World-space framing for the unit grenade orb at the origin.
///
/// Closer than the firearm kit: the charge is ~0.7 m across in preview, and a
/// gun-distance camera would leave an empty pane.
pub fn spin_reveal_grenade_focus() -> CameraFocus {
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-1.1, 0.55, 1.25), Vec3::ZERO)
}

pub const HEAD_ROOT_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "root", Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.05, 0.0));

pub const CROWN_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "crown_socket", Vec3::new(0.0, 0.15, 1.0), Vec3::ZERO);

/// Feature close-ups keep the whole face in a 45° preview so the edit reads
/// against the rest of the head.
pub const EYE_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "eye_socket.L", Vec3::new(0.0, 0.0, 0.5), Vec3::ZERO);

pub const NOSE_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "nose_socket", Vec3::new(0.0, 0.0, 0.45), Vec3::ZERO);

pub const MOUTH_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "mouth_socket", Vec3::new(0.0, 0.0, 0.45), Vec3::ZERO);

pub const EAR_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "ear_socket.L", Vec3::new(0.55, 0.0, 0.3), Vec3::ZERO);

// Head rigs inherit the body rig's asset normalization through the neck
// socket, so small species frame their heads at the same overall scale.

pub const TINY_HEAD_ROOT_FOCUS: CameraFocus = HEAD_ROOT_FOCUS.scaled(CHUPRI_OVERALL_SCALE);
pub const TINY_EYE_FOCUS: CameraFocus = EYE_FOCUS.scaled(CHUPRI_OVERALL_SCALE);
pub const TINY_MOUTH_FOCUS: CameraFocus = MOUTH_FOCUS.scaled(CHUPRI_OVERALL_SCALE);

pub const SMALL_BIRD_HEAD_ROOT_FOCUS: CameraFocus = HEAD_ROOT_FOCUS.scaled(TOPPLE_OVERALL_SCALE);
pub const SMALL_BIRD_EYE_FOCUS: CameraFocus = EYE_FOCUS.scaled(TOPPLE_OVERALL_SCALE);
pub const SMALL_BIRD_MOUTH_FOCUS: CameraFocus = MOUTH_FOCUS.scaled(TOPPLE_OVERALL_SCALE);

pub const KAPPLER_HEAD_ROOT_FOCUS: CameraFocus = HEAD_ROOT_FOCUS.scaled(KAPPLER_OVERALL_SCALE);
pub const KAPPLER_EYE_FOCUS: CameraFocus = EYE_FOCUS.scaled(KAPPLER_OVERALL_SCALE);
pub const KAPPLER_MOUTH_FOCUS: CameraFocus = MOUTH_FOCUS.scaled(KAPPLER_OVERALL_SCALE);

/// Chupri framing — closer for the ~1ft (~0.15×) creature.
pub const CHUPRI_BODY_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-0.2, 0.2, 0.8), Vec3::new(0.3, 0.05, -0.3));

/// Tipple framing — same close framing as Chupri (~1ft / 0.15×).
pub const TIPPLE_BODY_FOCUS: CameraFocus = CHUPRI_BODY_FOCUS;

/// Medium framing for ~2ft birds (Topple / Kispar at 0.30×).
pub const SMALL_BIRD_BODY_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-0.4, 0.4, 1.5), Vec3::new(0.6, 0.05, -0.5));

/// Spibmom framing — pulled back for the 2× head rig and long neck.
pub const SPIBMOM_BODY_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-1.2, 2.0, 10.0), Vec3::new(2.0, 0.0, -2.0));

/// Grener (~3 m shark) — BODY_FOCUS angle, pulled back for the larger silhouette.
pub const GRENER_BODY_FOCUS: CameraFocus = CameraFocus::new(
	FocusRig::Body,
	"dorsal_socket",
	Vec3::new(-1.5, 1.5, 7.0),
	Vec3::new(2.0, 0.0, -2.0),
);

/// Thumplus (~6 m whale) — same angle, backed up further.
pub const THUMPLUS_BODY_FOCUS: CameraFocus = CameraFocus::new(
	FocusRig::Body,
	"dorsal_socket",
	Vec3::new(-3.0, 3.0, 14.0),
	Vec3::new(4.0, 0.0, -4.0),
);

/// Mistler (~1 ft sprite fish) — BODY_FOCUS angle at Tipple/Chupri distance.
pub const MISTLER_BODY_FOCUS: CameraFocus = CameraFocus::new(
	FocusRig::Body,
	"dorsal_socket",
	Vec3::new(-0.3, 0.3, 1.2),
	Vec3::new(0.4, 0.05, -0.4),
);

pub const SPIBMOM_HEAD_ROOT_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "root", Vec3::new(0.0, 1.0, 7.0), Vec3::new(0.0, 0.05, 0.0));

pub const SPIBMOM_CROWN_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "crown_socket", Vec3::new(0.0, 0.15, 5.0), Vec3::ZERO);

pub const SPIBMOM_EYE_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "eye_socket.L", Vec3::new(0.0, 0.0, 3.1), Vec3::ZERO);

pub const SPIBMOM_NOSE_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "nose_socket", Vec3::new(0.0, 0.0, 3.1), Vec3::ZERO);
