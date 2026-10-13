//! Friend's Conifer joint foliage sizing ([#236](https://github.com/ramate-io/maybraid/issues/236), RFC §3.1.7.14).
//!
//! VegetationComponents uses Northern cheap-ball banding; these fractions size those clusters.

/// Needle-cluster world radius as a fraction of stalk height (RFC `0.018 * H`; denser than RFC for fuller Friend's silhouette).
pub const FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.028;

/// Local icosphere/plate sizing before joint scale (historical plane-splay defaults).
pub const FRIENDS_SPLAY_CORE_RADIUS: f32 = 0.85;
pub const FRIENDS_SPLAY_LEAF_DISC_RADIUS: f32 = 1.05;
