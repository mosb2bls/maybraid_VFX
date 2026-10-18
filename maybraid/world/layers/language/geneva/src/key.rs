//! Durable keys for names, and the salts derived from them.

use durham::GeographicFeatureId;
use lod::gen::Id;
use maybraid_language_core::lexicalizer::mix;

/// Key for a name owned by the language layer.
///
/// Domain is part of the key so the same numeric payload in another domain
/// cannot collide. ECS entity ids and rounded coordinates are not durable keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameKey {
	Region {
		ix: i32,
		iz: i32,
	},
	Forest(Id),
	Grove(Id),
	Urban(Id),
	UrbanLeaf(Id),
	Geographic(GeographicFeatureId),
	Place {
		host: Id,
		local: u32,
	},
	/// Rounded XZ + label when Richmond has not attached a host identity.
	/// Geneva names only identified places; the map keys these to no name.
	ProvisionalPlace {
		qx: i32,
		qz: i32,
		label: u32,
	},
}

pub fn name_key_salt(key: NameKey) -> u64 {
	let domain = match key {
		NameKey::Region { .. } => 0x01,
		NameKey::Forest(_) => 0x02,
		NameKey::Grove(_) => 0x03,
		NameKey::Urban(_) => 0x04,
		NameKey::UrbanLeaf(_) => 0x05,
		NameKey::Geographic(_) => 0x06,
		NameKey::Place { .. } => 0x07,
		NameKey::ProvisionalPlace { .. } => 0x08,
	};
	match key {
		NameKey::Region { ix, iz } => mix(domain) ^ mix(ix as u64) ^ mix(iz as u64),
		NameKey::Forest(id) | NameKey::Grove(id) | NameKey::Urban(id) | NameKey::UrbanLeaf(id) => {
			mix(domain) ^ mix(id_bits(id))
		}
		NameKey::Geographic(id) => {
			mix(domain)
				^ mix(id.family as u8 as u64)
				^ mix(id.band as u8 as u64)
				^ mix(id_bits(id.source))
		}
		NameKey::Place { host, local } => mix(domain) ^ mix(id_bits(host)) ^ mix(u64::from(local)),
		NameKey::ProvisionalPlace { qx, qz, label } => {
			mix(domain) ^ mix(qx as u64) ^ mix(qz as u64) ^ mix(u64::from(label))
		}
	}
}

fn id_bits(id: Id) -> u64 {
	match id {
		Id::Universal => 1,
		Id::Bytes(bytes) => {
			let mut out = 0u64;
			for chunk in bytes.0.chunks(8) {
				let mut buf = [0u8; 8];
				buf[..chunk.len()].copy_from_slice(chunk);
				out ^= u64::from_le_bytes(buf);
			}
			out
		}
		Id::OriginCell(cell) => {
			let bounds = cell.0 .0;
			u64::from(bounds.min.x.to_bits())
				^ u64::from(bounds.min.z.to_bits()).wrapping_shl(1)
				^ u64::from(bounds.max.x.to_bits()).wrapping_shl(2)
		}
	}
}
