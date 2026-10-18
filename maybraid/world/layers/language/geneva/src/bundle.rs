//! Grammar + lexicon bundles composed for a large tile.

use maybraid_language_core::lexicalizer::mix;
use maybraid_language_grammars::{CompositeGrammar, WordOrder};

/// Which lexicalizer family a tile language uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LexiconFamily {
	Compositional,
	RootHeavy,
}

/// Concrete `(grammar, lexicon)` realized through language-core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageBundle {
	pub grammar: CompositeGrammar,
	pub lexicon: LexiconFamily,
	pub seed: u64,
}

impl LanguageBundle {
	pub fn compose(world_seed: u64, coarse: (i32, i32), local: (i32, i32), index: u8) -> Self {
		let coarse_hash = mix(world_seed
			^ mix(coarse.0 as u64)
			^ mix((coarse.1 as u64).wrapping_mul(0x9E37))
			^ u64::from(index));
		let local_hash = mix(world_seed
			^ mix(local.0 as u64)
			^ mix((local.1 as u64).wrapping_mul(0x85EB))
			^ u64::from(index)
			^ 0xD1E7_A11A);
		let presets = grammar_presets();
		let mut grammar = presets[(coarse_hash as usize) % presets.len()];
		if local_hash.is_multiple_of(4) {
			grammar.order = WORD_ORDERS[((local_hash >> 3) as usize) % WORD_ORDERS.len()];
		}
		let lexicon = if local_hash.is_multiple_of(2) {
			LexiconFamily::Compositional
		} else {
			LexiconFamily::RootHeavy
		};
		Self { grammar, lexicon, seed: mix(local_hash ^ 0xA11A_B22B) }
	}
}

const WORD_ORDERS: [WordOrder; 6] = [
	WordOrder::Svo,
	WordOrder::Sov,
	WordOrder::Vso,
	WordOrder::Vos,
	WordOrder::Ovs,
	WordOrder::Osv,
];

fn grammar_presets() -> [CompositeGrammar; 8] {
	[
		CompositeGrammar::isolating_svo(),
		CompositeGrammar::agglutinative_sov(),
		CompositeGrammar::fusional_svo(),
		CompositeGrammar::particle_heavy_topic_prominent(),
		CompositeGrammar::serial_verb(),
		CompositeGrammar::ergative_vso(),
		CompositeGrammar::separable_svo(),
		CompositeGrammar::basic_compositional(),
	]
}
