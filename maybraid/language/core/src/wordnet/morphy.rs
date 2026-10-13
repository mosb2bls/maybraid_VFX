//! WordNet Morphy: exception lists plus suffix substitutions.
//!
//! This is the dictionary morphological preprocessor, not a second parser.
//! UDPipe still owns syntax; this only maps surface/UD lemmas onto citation
//! forms that `index.*` actually contains.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::concept::Pos;
use crate::error::LanguageError;
use crate::wordnet::normalize_lemma;

/// Irregulars from `*.exc` and regular suffix rewrites from WordNet 3.1.
#[derive(Clone, Debug, Default)]
pub struct Morphy {
	exceptions: HashMap<Pos, HashMap<String, Vec<String>>>,
}

impl Morphy {
	pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self, LanguageError> {
		let dir = dir.as_ref();
		let mut exceptions = HashMap::new();
		for (pos, file) in [
			(Pos::Noun, "noun.exc"),
			(Pos::Verb, "verb.exc"),
			(Pos::Adjective, "adj.exc"),
			(Pos::Adverb, "adv.exc"),
		] {
			exceptions.insert(pos, load_exceptions(&dir.join(file))?);
		}
		Ok(Self { exceptions })
	}

	/// Citation-form candidates for `form` in `pos`, WordNet Morphy order.
	///
	/// Exception hits replace suffix guessing. Otherwise the original form
	/// plus one-step suffix substitutions are returned; the caller looks
	/// those keys up in `index.*`.
	pub fn candidates(&self, form: &str, pos: Pos) -> Vec<String> {
		let form = normalize_lemma(form);
		if form.is_empty() {
			return Vec::new();
		}
		let pos = morph_pos(pos);
		if let Some(bases) = self.exceptions.get(&pos).and_then(|map| map.get(&form)) {
			return bases.clone();
		}
		let mut out = vec![form.clone()];
		for candidate in suffix_substitutions(&form, pos) {
			if !out.contains(&candidate) {
				out.push(candidate);
			}
		}
		if pos == Pos::Noun {
			if let Some(stem) = form.strip_suffix("ful") {
				if !stem.is_empty() {
					for candidate in suffix_substitutions(stem, Pos::Noun) {
						let restored = format!("{candidate}ful");
						if !out.contains(&restored) {
							out.push(restored);
						}
					}
				}
			}
		}
		out
	}
}

fn morph_pos(pos: Pos) -> Pos {
	match pos {
		Pos::AdjectiveSatellite => Pos::Adjective,
		other => other,
	}
}

fn load_exceptions(path: &Path) -> Result<HashMap<String, Vec<String>>, LanguageError> {
	let text = fs::read_to_string(path).map_err(|error| LanguageError::WordNet {
		path: path.display().to_string(),
		detail: error.to_string(),
	})?;
	let mut map = HashMap::new();
	for line in text.lines() {
		let mut parts = line.split_whitespace();
		let Some(inflected) = parts.next() else {
			continue;
		};
		let bases: Vec<String> =
			parts.map(normalize_lemma).filter(|base| !base.is_empty()).collect();
		if !bases.is_empty() {
			map.insert(normalize_lemma(inflected), bases);
		}
	}
	Ok(map)
}

/// Official WordNet 3.1 suffix / ending pairs from `morph.c`.
fn suffix_substitutions(form: &str, pos: Pos) -> Vec<String> {
	let rules: &[(&str, &str)] = match pos {
		Pos::Noun => &[
			("s", ""),
			("ses", "s"),
			("xes", "x"),
			("zes", "z"),
			("ches", "ch"),
			("shes", "sh"),
			("men", "man"),
			("ies", "y"),
		],
		Pos::Verb => &[
			("s", ""),
			("ies", "y"),
			("es", "e"),
			("es", ""),
			("ed", "e"),
			("ed", ""),
			("ing", "e"),
			("ing", ""),
		],
		Pos::Adjective | Pos::AdjectiveSatellite => {
			&[("er", ""), ("est", ""), ("er", "e"), ("est", "e")]
		}
		Pos::Adverb => &[],
	};
	let mut out = Vec::new();
	for (suffix, ending) in rules {
		if let Some(stem) = form.strip_suffix(suffix) {
			if stem.is_empty() {
				continue;
			}
			let candidate = format!("{stem}{ending}");
			if candidate.len() >= 2 && !out.contains(&candidate) {
				out.push(candidate);
			}
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::wordnet::dict::bundled_dict_dir;

	#[test]
	fn exceptions_cover_irregulars() -> Result<(), LanguageError> {
		let morphy = Morphy::from_dir(bundled_dict_dir())?;
		assert_eq!(morphy.candidates("struck", Pos::Verb), vec!["strike".to_owned()]);
		assert_eq!(morphy.candidates("children", Pos::Noun), vec!["child".to_owned()]);
		assert_eq!(morphy.candidates("gave", Pos::Verb), vec!["give".to_owned()]);
		assert_eq!(morphy.candidates("went", Pos::Verb), vec!["go".to_owned()]);
		assert!(morphy.candidates("better", Pos::Adjective).contains(&"good".to_owned()));
		Ok(())
	}

	#[test]
	fn suffix_rules_reduce_regular_inflections() -> Result<(), LanguageError> {
		let morphy = Morphy::from_dir(bundled_dict_dir())?;
		assert!(morphy.candidates("strikes", Pos::Verb).contains(&"strike".to_owned()));
		assert!(morphy.candidates("boxes", Pos::Noun).contains(&"box".to_owned()));
		assert!(morphy.candidates("walking", Pos::Verb).contains(&"walk".to_owned()));
		assert!(morphy.candidates("watched", Pos::Verb).contains(&"watch".to_owned()));
		Ok(())
	}
}
