//! Small parser-backed argument value types shared by procedural front-ends.

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitRange {
	pub start: f32,
	pub end: f32,
}

impl UnitRange {
	pub const fn new(start: f32, end: f32) -> Self {
		Self { start, end }
	}

	pub fn as_range(self) -> Range<f32> {
		self.start..self.end
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CountPair {
	pub first: u32,
	pub second: u32,
}

impl CountPair {
	pub const fn new(first: u32, second: u32) -> Self {
		Self { first, second }
	}
}

/// Parse `left<delimiter>right` after trimming each side.
fn parse_delimited_pair<T>(
	s: &str,
	delimiter: &str,
	expected_shape: &str,
	parse_item: impl Fn(&str) -> Result<T, String>,
) -> Result<(T, T), String> {
	let (left, right) = s
		.split_once(delimiter)
		.ok_or_else(|| format!("expected {expected_shape}, got {s:?}"))?;
	Ok((parse_item(left.trim())?, parse_item(right.trim())?))
}

macro_rules! define_int_range {
	($name:ident, $ty:ty, $parse_fn:ident) => {
		#[derive(Clone, Copy, Debug, PartialEq, Eq)]
		pub struct $name {
			pub start: $ty,
			pub end: $ty,
		}

		impl $name {
			pub const fn new(start: $ty, end: $ty) -> Self {
				Self { start, end }
			}

			pub fn as_range(self) -> Range<$ty> {
				self.start..self.end
			}
		}

		impl From<$name> for Range<$ty> {
			fn from(value: $name) -> Self {
				value.as_range()
			}
		}

		pub fn $parse_fn(s: &str) -> Result<$name, String> {
			let (start, end) = parse_delimited_pair(s, "..", "start..end range", |t| {
				t.parse::<$ty>().map_err(|e| e.to_string())
			})?;
			Ok($name::new(start, end))
		}
	};
}

define_int_range!(U32Range, u32, parse_u32_range);
define_int_range!(UsizeRange, usize, parse_usize_range);

pub fn parse_unit_range(s: &str) -> Result<UnitRange, String> {
	let (start, end) = parse_delimited_pair(s, "..", "start..end range", |t| {
		t.parse::<f32>().map_err(|e| e.to_string())
	})?;
	Ok(UnitRange::new(start, end))
}

pub fn parse_count_pair(s: &str) -> Result<CountPair, String> {
	let (first, second) = parse_delimited_pair(s, "x", "firstxsecond counts", |t| {
		t.parse::<u32>().map_err(|e| e.to_string())
	})?;
	Ok(CountPair::new(first, second))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_common_arg_shapes() -> anyhow::Result<()> {
		assert_eq!(
			parse_unit_range("0.40..0.95").map_err(|e| anyhow::anyhow!("{e}"))?,
			UnitRange::new(0.40, 0.95)
		);
		assert_eq!(
			parse_count_pair("8x7").map_err(|e| anyhow::anyhow!("{e}"))?,
			CountPair::new(8, 7)
		);
		assert_eq!(
			parse_u32_range("2..5").map_err(|e| anyhow::anyhow!("{e}"))?,
			U32Range::new(2, 5)
		);
		assert_eq!(
			parse_usize_range("4..6").map_err(|e| anyhow::anyhow!("{e}"))?,
			UsizeRange::new(4, 6)
		);
		Ok(())
	}

	#[test]
	fn int_ranges_convert_to_std_range() -> anyhow::Result<()> {
		let u32_range: Range<u32> = U32Range::new(1, 3).into();
		assert_eq!(u32_range, 1..3);
		let usize_range: Range<usize> = UsizeRange::new(0, 1).into();
		assert_eq!(usize_range, 0..1);
		Ok(())
	}
}
