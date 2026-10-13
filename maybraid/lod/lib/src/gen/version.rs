/// Monotonic per-store storage version.
///
/// Stamped by the store on every insert (including re-inserts that overwrite
/// an id). Presenters compare a stored version against the version they last
/// presented per id, so "genuinely new" and "changed since presented" are
/// knowable from data alone — no commit phase or transient event handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(pub u64);
