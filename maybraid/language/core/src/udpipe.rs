//! UDPipe-backed [`EnglishDependencyParser`].

use std::path::{Path, PathBuf};

use udpipe_rs::Model;

use crate::error::LanguageError;
use crate::parse::{
	DependencyDocument, DependencyRelation, DependencySentence, DependencyToken,
	EnglishDependencyParser, MorphFeatures, TokenId, UniversalPos,
};

/// Bundled English EWT model. Not embedded; load from disk.
pub const BUNDLED_UDPIPE_FILE: &str = "english-ewt.udpipe";

pub fn bundled_udpipe_path() -> PathBuf {
	if let Ok(path) = std::env::var("MAYBRAID_UDPIPE") {
		return PathBuf::from(path);
	}
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../assets/language/udpipe")
		.join(BUNDLED_UDPIPE_FILE)
}

/// Offline English parser over a local `.udpipe` model.
pub struct UdpipeEnglishParser {
	model: Model,
}

impl UdpipeEnglishParser {
	pub fn from_path(path: impl AsRef<Path>) -> Result<Self, LanguageError> {
		let path = path.as_ref();
		if !path.is_file() {
			return Err(LanguageError::UdpipeMissing { path: path.display().to_string() });
		}
		let model = Model::load(path).map_err(|error| {
			LanguageError::DependencyParse(format!("failed to load {}: {error}", path.display()))
		})?;
		Ok(Self { model })
	}

	pub fn bundled() -> Result<Self, LanguageError> {
		Self::from_path(bundled_udpipe_path())
	}
}

impl EnglishDependencyParser for UdpipeEnglishParser {
	fn parse(&self, text: &str) -> Result<DependencyDocument, LanguageError> {
		let words = self
			.model
			.parse(text)
			.map_err(|error| LanguageError::DependencyParse(error.to_string()))?;
		Ok(document_from_words(&words))
	}
}

fn document_from_words(words: &[udpipe_rs::Word]) -> DependencyDocument {
	let mut sentences = Vec::new();
	let mut current_id = None;
	let mut current = DependencySentence::default();
	for word in words {
		if word.is_punct() && word.form == "_" {
			continue;
		}
		if current_id.is_some()
			&& current_id != Some(word.sentence_id)
			&& !current.tokens.is_empty()
		{
			sentences.push(std::mem::take(&mut current));
		}
		current_id = Some(word.sentence_id);
		if word.id <= 0 {
			continue;
		}
		current.tokens.push(DependencyToken {
			id: TokenId(word.id as u32),
			text: word.form.clone(),
			lemma: if word.lemma.is_empty() || word.lemma == "_" {
				word.form.clone()
			} else {
				word.lemma.clone()
			},
			pos: UniversalPos::parse(&word.upostag),
			features: MorphFeatures::parse(&word.feats),
			head: if word.head > 0 { Some(TokenId(word.head as u32)) } else { None },
			relation: DependencyRelation::parse(word.deprel.clone()),
		});
	}
	if !current.tokens.is_empty() {
		sentences.push(current);
	}
	DependencyDocument { sentences }
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::parse::EnglishDependencyParser;

	#[test]
	fn bundled_path_is_under_language_assets() {
		let path = bundled_udpipe_path();
		assert!(
			path.to_string_lossy().contains("assets/language/udpipe")
				|| std::env::var("MAYBRAID_UDPIPE").is_ok(),
			"bundled path should live under assets/language/udpipe, got {}",
			path.display()
		);
		assert!(path.ends_with(BUNDLED_UDPIPE_FILE) || std::env::var("MAYBRAID_UDPIPE").is_ok());
	}

	#[test]
	fn missing_model_is_a_typed_error() -> Result<(), LanguageError> {
		let error =
			UdpipeEnglishParser::from_path("/no/such/english-ewt.udpipe").err().ok_or_else(
				|| LanguageError::DependencyParse("missing model should fail".to_owned()),
			)?;
		assert!(matches!(error, LanguageError::UdpipeMissing { .. }));
		Ok(())
	}

	#[test]
	fn live_parse_john_gave_mary_the_book() -> Result<(), LanguageError> {
		let Ok(parser) = UdpipeEnglishParser::bundled() else {
			return Ok(());
		};
		let document = parser.parse("John gave Mary the book.")?;
		assert!(!document.sentences.is_empty());
		let tokens = &document.sentences[0].tokens;
		assert!(tokens.iter().any(|token| token.lemma == "give" && token.relation.is("root")));
		assert!(tokens.iter().any(|token| token.text == "John" && token.relation.is("nsubj")));
		Ok(())
	}
}
