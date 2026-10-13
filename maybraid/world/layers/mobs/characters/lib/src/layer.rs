//! Provenance layers for [`crate::CharacterComponents`] node maps.
//!
//! A [`Layer`] is **not** a node-type tag (rig vs part). Domain type stays on the
//! trait method (`rig_nodes_for_level`, `part_nodes_for_level`). Layer records
//! where geometry came from so parents can apply policy. Prefer [`Layers::free`]
//! until a provenance name is meaningful. Characters compose by [`Layers::extend`].

use std::collections::{HashMap, HashSet};
use std::fmt;

/// Provenance label for a bucket of authored geometry.
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
		Self(value)
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

	pub fn from_labeled(layer: impl Into<Layer>, nodes: Vec<T>) -> Self {
		let mut layers = Self::new();
		layers.extend_labeled(layer, nodes);
		layers
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

	pub fn with_free(mut self, nodes: impl IntoIterator<Item = T>) -> Self {
		self.extend_free(nodes);
		self
	}

	pub fn push_labeled(&mut self, layer: impl Into<Layer>, node: T) -> &mut Self {
		self.labeled.entry(layer.into()).or_default().push(node);
		self
	}

	pub fn extend_labeled(
		&mut self,
		layer: impl Into<Layer>,
		nodes: impl IntoIterator<Item = T>,
	) -> &mut Self {
		self.labeled.entry(layer.into()).or_default().extend(nodes);
		self
	}

	pub fn with_labeled(
		mut self,
		layer: impl Into<Layer>,
		nodes: impl IntoIterator<Item = T>,
	) -> Self {
		self.extend_labeled(layer, nodes);
		self
	}

	/// Flatten `other` and append under a single provenance label.
	pub fn extend_under(&mut self, layer: impl Into<Layer>, other: Layers<T>) -> &mut Self {
		self.extend_labeled(layer, other.flatten())
	}

	/// Merge another [`Layers`], appending free lists and same-named labels.
	pub fn extend(&mut self, other: Layers<T>) -> &mut Self {
		self.free.extend(other.free);
		for (layer, mut nodes) in other.labeled {
			self.labeled.entry(layer).or_default().append(&mut nodes);
		}
		self
	}

	pub fn extended(mut self, other: Layers<T>) -> Self {
		self.extend(other);
		self
	}

	pub fn except(self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
		let deny: HashSet<String> = names.into_iter().map(|n| n.as_ref().to_string()).collect();
		Self {
			free: self.free,
			labeled: self
				.labeled
				.into_iter()
				.filter(|(layer, _)| !deny.contains(layer.as_str()))
				.collect(),
		}
	}

	pub fn only(self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
		let allow: HashSet<String> = names.into_iter().map(|n| n.as_ref().to_string()).collect();
		Self {
			free: Vec::new(),
			labeled: self
				.labeled
				.into_iter()
				.filter(|(layer, _)| allow.contains(layer.as_str()))
				.collect(),
		}
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

impl<T> Extend<T> for Layers<T> {
	fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
		self.extend_free(iter);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn extend_under_flattens_into_label() {
		let mut out = Layers::new();
		let child = Layers::new().with_free([1]).with_labeled("features", [2]);
		out.extend_under("clothing", child);
		assert!(out.free.is_empty());
		assert_eq!(out.labeled.get(&Layer::new("clothing")).unwrap(), &vec![1, 2]);
	}
}
