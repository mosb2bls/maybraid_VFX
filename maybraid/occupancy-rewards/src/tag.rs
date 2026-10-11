//! Stable hashed tags so rules can name affiliation, grove kind, and
//! construction without depending on Chico or Bikanwarda enums.

const FNV_OFFSET: u32 = 0x811c9dc5;
const FNV_PRIME: u32 = 0x0100_0193;

pub const fn tag_hash(name: &str) -> u32 {
	let bytes = name.as_bytes();
	let mut hash = FNV_OFFSET;
	let mut i = 0;
	while i < bytes.len() {
		hash ^= bytes[i] as u32;
		hash = hash.wrapping_mul(FNV_PRIME);
		i += 1;
	}
	hash
}

macro_rules! semantic_tag {
	($name:ident, $doc:expr) => {
		#[doc = $doc]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
		pub struct $name(pub u32);

		impl $name {
			pub const fn named(name: &'static str) -> Self {
				Self(tag_hash(name))
			}
		}
	};
}

semantic_tag!(AffiliationTag, "Publisher-defined cell affiliation.");
semantic_tag!(LayeringTag, "Forest layering identity (Hopscotch / `LayeringKind`).");
semantic_tag!(GroveKindTag, "Grove recipe identity (`ForestGroveKind`).");
semantic_tag!(ConstructionTag, "Named construction present in a semantic cell.");

impl ConstructionTag {
	/// Sope's Banyan. A well-known vegetation construction, not a cell id.
	pub const SOPES_BANYAN: Self = Self::named("sopes_banyan");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn named_tags_are_stable() -> anyhow::Result<()> {
		assert_eq!(ConstructionTag::named("sopes_banyan"), ConstructionTag::SOPES_BANYAN);
		assert_ne!(GroveKindTag::named("TradeWinds"), GroveKindTag::named("Orchard"));
		Ok(())
	}
}
