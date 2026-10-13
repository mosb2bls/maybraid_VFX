/// Collapsible section open/closed state keyed by section label.
pub trait SectionOpen {
	fn is_open(&self, label: &'static str) -> bool;
}
