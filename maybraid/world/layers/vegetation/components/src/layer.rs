//! Provenance layers for [`crate::VegetationComponents`] node maps.

use std::collections::HashMap;
use std::fmt;

/// Provenance label for a bucket of authored geometry (not node-type identity).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Layer(pub String);

impl Layer {
	pub fn new(name: impl Into<String>) -> Self {
		Self(name.into())
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl From<&str> for Layer {
	fn from(value: &str) -> Self {
		Self::new(value)
	}
}

impl From<String> for Layer {
	fn from(value: String) -> Self {
		Self::new(value)
	}
}

impl AsRef<str> for Layer {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

impl fmt::Display for Layer {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

/// Nodes with optional provenance labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layers<T> {
	pub free: Vec<T>,
	pub labeled: HashMap<Layer, Vec<T>>,
}

impl<T> Default for Layers<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T> Layers<T> {
	pub fn new() -> Self {
		Self { free: Vec::new(), labeled: HashMap::new() }
	}

	pub fn from_free(nodes: Vec<T>) -> Self {
		Self { free: nodes, labeled: HashMap::new() }
	}

	pub fn is_empty(&self) -> bool {
		self.free.is_empty() && self.labeled.values().all(|v| v.is_empty())
	}

	pub fn len(&self) -> usize {
		self.free.len() + self.labeled.values().map(|v| v.len()).sum::<usize>()
	}

	pub fn push_free(&mut self, node: T) -> &mut Self {
		self.free.push(node);
		self
	}

	pub fn extend_free(&mut self, nodes: impl IntoIterator<Item = T>) -> &mut Self {
		self.free.extend(nodes);
		self
	}

	pub fn extend(&mut self, other: Layers<T>) -> &mut Self {
		self.free.extend(other.free);
		for (layer, mut nodes) in other.labeled {
			self.labeled.entry(layer).or_default().append(&mut nodes);
		}
		self
	}

	pub fn flatten(self) -> Vec<T> {
		let mut out = self.free;
		let mut entries: Vec<_> = self.labeled.into_iter().collect();
		entries.sort_by(|a, b| a.0.cmp(&b.0));
		for (_, nodes) in entries {
			out.extend(nodes);
		}
		out
	}

	/// Map every free and labeled node.
	pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Layers<U> {
		Layers {
			free: self.free.into_iter().map(&mut f).collect(),
			labeled: self
				.labeled
				.into_iter()
				.map(|(layer, nodes)| (layer, nodes.into_iter().map(&mut f).collect()))
				.collect(),
		}
	}
}

impl<T> FromIterator<T> for Layers<T> {
	fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
		Self::from_free(iter.into_iter().collect())
	}
}
