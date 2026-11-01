// SPDX-License-Identifier: CC0-1.0
//
// ported from rocicorp/fractional-indexing v4.0.0
// (https://github.com/rocicorp/fractional-indexing), based on David Greenspan's
// "Implementing Fractional Indexing"
// (https://observablehq.com/@dgreensp/implementing-fractional-indexing).
// Only the default-format path was ported: BASE_62 digit values with BASE_52
// integer heads.

use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

pub const BASE_62_DIGITS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BASE_52_DIGITS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

const ZERO: u8 = b'0';

/// A base-62 digit's value is its index in `BASE_62_DIGITS`, not its ASCII code:
/// `'0'` is 0, `'A'` is 10, `'a'` is 36, and `'z'` is 61.
fn digit_value(byte: u8) -> Option<usize> {
    match byte {
        b'0'..=b'9' => Some((byte - b'0') as usize),
        b'A'..=b'Z' => Some((byte - b'A') as usize + 10),
        b'a'..=b'z' => Some((byte - b'a') as usize + 36),
        _ => None,
    }
}

/// Inverse of `digit_value`, used when emitting a digit.
fn digit_char(index: usize) -> Option<char> {
    BASE_62_DIGITS
        .as_bytes()
        .get(index)
        .map(|&byte| byte as char)
}

/// Maps a size letter (`A-Z`, `a-z`) to its position in `BASE_52_DIGITS`. This is
/// only for deriving the integer-part length, not base-62 digit arithmetic.
fn int_digit_value(byte: u8) -> Option<usize> {
    match byte {
        b'A'..=b'Z' => Some((byte - b'A') as usize),
        b'a'..=b'z' => Some((byte - b'a') as usize + 26),
        _ => None,
    }
}

/// The size letter encodes the integer-part length: two characters at the middle
/// of `BASE_52_DIGITS`, one more per step outward.
fn integer_length_for_head(head: u8) -> Option<usize> {
    let index = int_digit_value(head)?;
    let half = BASE_52_DIGITS.len() / 2;
    Some(if index < half {
        half - index + 1
    } else {
        index - half + 2
    })
}

/// Reads the integer-part length from the key's first character.
fn get_integer_length(head: &str) -> Result<usize, PositionError> {
    let byte = head
        .as_bytes()
        .first()
        .copied()
        .ok_or_else(|| PositionError::InvalidKey("invalid order key head: ".to_string()))?;
    integer_length_for_head(byte).ok_or_else(|| {
        PositionError::InvalidKey(format!("invalid order key head: {}", byte as char))
    })
}

/// Splits a key into its integer part and fraction, using the size letter.
fn get_integer_part(key: &str) -> Result<&str, PositionError> {
    let length = get_integer_length(key)?;
    if length > key.len() {
        return Err(PositionError::InvalidKey(format!(
            "invalid order key: {key}"
        )));
    }
    Ok(key.get(..length).unwrap_or(""))
}

/// The most negative integer is reserved: accepting it would leave no key that
/// can be generated before it.
fn is_smallest_integer(key: &str) -> bool {
    let zero_count = BASE_52_DIGITS.len() / 2;
    key.len() == zero_count + 1
        && key.starts_with('A')
        && key.as_bytes()[1..].iter().all(|&byte| byte == ZERO)
}

/// Checks the format invariants: known digits, a valid size letter, a long
/// enough integer part, and a fraction that does not end in `0`.
fn validate_order_key(key: &str) -> Result<(), PositionError> {
    if key.is_empty() {
        return Err(PositionError::InvalidKey(format!(
            "invalid order key: {key}"
        )));
    }
    if !key.bytes().all(|byte| digit_value(byte).is_some()) {
        return Err(PositionError::InvalidKey(format!(
            "invalid order key: {key}"
        )));
    }
    if is_smallest_integer(key) {
        return Err(PositionError::InvalidKey(format!(
            "invalid order key: {key}"
        )));
    }
    let integer_part = get_integer_part(key)?;
    let fraction = key.get(integer_part.len()..).unwrap_or("");
    if fraction.as_bytes().last() == Some(&ZERO) {
        return Err(PositionError::InvalidKey(format!(
            "invalid order key: {key}"
        )));
    }
    Ok(())
}

/// Checks that an integer part is exactly as long as its size letter requires.
fn validate_integer(integer: &str) -> Result<(), PositionError> {
    if integer.len() != get_integer_length(integer)? {
        return Err(PositionError::InvalidKey(format!(
            "invalid integer part of order key: {integer}"
        )));
    }
    Ok(())
}

/// Finds a digit string strictly between two fractions, or above `a` when `b`
/// is `None`. Consecutive first digits keep the lower digit and recurse, which
/// is what buys room by lengthening the key.
fn midpoint(a: &str, b: Option<&str>) -> Result<String, PositionError> {
    if let Some(b) = b {
        if a >= b {
            return Err(PositionError::InvalidKey(format!("{a} >= {b}")));
        }
        if a.as_bytes().last() == Some(&ZERO) || b.as_bytes().last() == Some(&ZERO) {
            return Err(PositionError::InvalidKey("trailing zero".to_string()));
        }

        // Remove the longest common prefix, treating a short `a` as zero
        // padded. `b` cannot end before `a` here, so it needs no padding.
        let mut shared = 0;
        loop {
            let a_digit = a.as_bytes().get(shared).copied().unwrap_or(ZERO);
            match b.as_bytes().get(shared).copied() {
                Some(b_digit) if b_digit == a_digit => shared += 1,
                _ => break,
            }
        }
        if shared > 0 {
            let prefix = b.get(..shared).unwrap_or("");
            let rest = midpoint(
                a.get(shared..).unwrap_or(""),
                Some(b.get(shared..).unwrap_or("")),
            )?;
            return Ok(format!("{prefix}{rest}"));
        }
    } else if a.as_bytes().last() == Some(&ZERO) {
        return Err(PositionError::InvalidKey("trailing zero".to_string()));
    }

    let digit_a = match a.as_bytes().first().copied() {
        Some(byte) => digit_value(byte).ok_or_else(|| {
            PositionError::InvalidKey(format!("invalid order key: {}", byte as char))
        })?,
        None => 0,
    };
    let digit_b = match b.and_then(|b| b.as_bytes().first().copied()) {
        Some(byte) => digit_value(byte).ok_or_else(|| {
            PositionError::InvalidKey(format!("invalid order key: {}", byte as char))
        })?,
        None => BASE_62_DIGITS.len(),
    };

    if digit_b > digit_a + 1 {
        // integer arithmetic matches Math.round(0.5 * (digit_a + digit_b))
        let mid = (digit_a + digit_b).div_ceil(2);
        let mid_char = digit_char(mid)
            .ok_or_else(|| PositionError::InvalidKey("invalid order key".to_string()))?;
        return Ok(mid_char.to_string());
    }

    match b {
        Some(b) if b.len() > 1 => return Ok(b.get(..1).unwrap_or("").to_string()),
        _ => {}
    }

    let first = digit_char(digit_a)
        .ok_or_else(|| PositionError::InvalidKey("invalid order key".to_string()))?;
    let rest = midpoint(a.get(1..).unwrap_or(""), None)?;
    Ok(format!("{first}{rest}"))
}

/// Returns the next integer part, carrying into a longer size letter when the
/// digit run is exhausted. `None` means the integer part is already the largest.
fn increment_integer(integer: &str) -> Result<Option<String>, PositionError> {
    validate_integer(integer)?;
    let bytes = integer.as_bytes();
    let head = bytes[0] as char;
    let mut trailing = String::new();
    for i in (1..bytes.len()).rev() {
        let digit = digit_value(bytes[i])
            .ok_or_else(|| PositionError::InvalidKey(format!("invalid order key: {integer}")))?;
        if digit + 1 == BASE_62_DIGITS.len() {
            trailing.insert(0, '0');
        } else {
            let prefix = integer.get(1..i).unwrap_or("");
            let next = digit_char(digit + 1)
                .ok_or_else(|| PositionError::InvalidKey("invalid order key".to_string()))?;
            return Ok(Some(format!("{head}{prefix}{next}{trailing}")));
        }
    }

    let head_index = int_digit_value(bytes[0])
        .ok_or_else(|| PositionError::InvalidKey(format!("invalid order key head: {head}")))?;
    if head_index == BASE_52_DIGITS.len() - 1 {
        return Ok(None);
    }
    let next_head = BASE_52_DIGITS.as_bytes()[head_index + 1] as char;
    let length_delta = integer_length_for_head(next_head as u8).unwrap_or_default() as i32
        - integer_length_for_head(bytes[0]).unwrap_or_default() as i32;
    let tail = match length_delta.cmp(&0) {
        Ordering::Greater => format!("{trailing}0"),
        Ordering::Less => trailing.get(1..).unwrap_or("").to_string(),
        Ordering::Equal => trailing,
    };
    Ok(Some(format!("{next_head}{tail}")))
}

/// Returns the previous integer part, borrowing into a longer negative size
/// letter when the digit run is exhausted. `None` means it is already the
/// smallest.
fn decrement_integer(integer: &str) -> Result<Option<String>, PositionError> {
    validate_integer(integer)?;
    let bytes = integer.as_bytes();
    let head = bytes[0] as char;
    let mut trailing = String::new();
    for i in (1..bytes.len()).rev() {
        let digit = digit_value(bytes[i])
            .ok_or_else(|| PositionError::InvalidKey(format!("invalid order key: {integer}")))?;
        if digit == 0 {
            trailing.insert(0, 'z');
        } else {
            let prefix = integer.get(1..i).unwrap_or("");
            let previous = digit_char(digit - 1)
                .ok_or_else(|| PositionError::InvalidKey("invalid order key".to_string()))?;
            return Ok(Some(format!("{head}{prefix}{previous}{trailing}")));
        }
    }

    let head_index = int_digit_value(bytes[0])
        .ok_or_else(|| PositionError::InvalidKey(format!("invalid order key head: {head}")))?;
    if head_index == 0 {
        return Ok(None);
    }
    let previous_head = BASE_52_DIGITS.as_bytes()[head_index - 1] as char;
    let length_delta = integer_length_for_head(previous_head as u8).unwrap_or_default() as i32
        - integer_length_for_head(bytes[0]).unwrap_or_default() as i32;
    let tail = match length_delta.cmp(&0) {
        Ordering::Greater => format!("{trailing}z"),
        Ordering::Less => trailing.get(1..).unwrap_or("").to_string(),
        Ordering::Equal => trailing,
    };
    Ok(Some(format!("{previous_head}{tail}")))
}

/// Port of the reference `generateKeyBetween`: either bound may be absent, and
/// reversed bounds are normalized to a lower/upper pair.
fn generate_key_between(a: Option<&str>, b: Option<&str>) -> Result<String, PositionError> {
    if let Some(a) = a {
        validate_order_key(a)?;
    }
    if let Some(b) = b {
        validate_order_key(b)?;
    }

    // reversed bounds are normalized; equal bounds cannot produce a key
    let (a, b) = match (a, b) {
        (Some(a), Some(b)) if a > b => (Some(b), Some(a)),
        bounds => bounds,
    };
    match (a, b) {
        (Some(a_key), Some(b_key)) if a_key == b_key => {
            return Err(PositionError::InvalidBounds {
                before: a_key.to_string(),
                after: b_key.to_string(),
            });
        }
        _ => {}
    }

    let result = match (a, b) {
        (None, None) => {
            let head = BASE_52_DIGITS.as_bytes()[BASE_52_DIGITS.len() / 2] as char;
            format!("{head}0")
        }
        (None, Some(b)) => {
            let integer_b = get_integer_part(b)?;
            let fraction_b = b.get(integer_b.len()..).unwrap_or("");
            if is_smallest_integer(integer_b) {
                format!("{integer_b}{}", midpoint("", Some(fraction_b))?)
            } else if integer_b < b {
                integer_b.to_string()
            } else {
                decrement_integer(integer_b)?.ok_or(PositionError::CannotDecrement)?
            }
        }
        (Some(a), None) => {
            let integer_a = get_integer_part(a)?;
            let fraction_a = a.get(integer_a.len()..).unwrap_or("");
            match increment_integer(integer_a)? {
                Some(incremented) => incremented,
                None => format!("{integer_a}{}", midpoint(fraction_a, None)?),
            }
        }
        (Some(a), Some(b)) => {
            let integer_a = get_integer_part(a)?;
            let fraction_a = a.get(integer_a.len()..).unwrap_or("");
            let integer_b = get_integer_part(b)?;
            let fraction_b = b.get(integer_b.len()..).unwrap_or("");
            if integer_a == integer_b {
                format!("{integer_a}{}", midpoint(fraction_a, Some(fraction_b))?)
            } else {
                match increment_integer(integer_a)? {
                    None => return Err(PositionError::CannotIncrement),
                    Some(incremented) if incremented.as_str() < b => incremented,
                    Some(_) => format!("{integer_a}{}", midpoint(fraction_a, None)?),
                }
            }
        }
    };

    validate_order_key(&result)?;
    Ok(result)
}

/// Port of `generateNKeysBetween`: split the interval at its midpoint and recurse
/// so keys are spread out instead of chained off one end.
fn generate_n_keys_between(
    a: Option<&str>,
    b: Option<&str>,
    n: usize,
) -> Result<Vec<String>, PositionError> {
    if n == 0 {
        return Ok(Vec::new());
    }
    if n == 1 {
        return Ok(vec![generate_key_between(a, b)?]);
    }
    if b.is_none() {
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            let current = if i == 0 {
                generate_key_between(a, b)?
            } else {
                generate_key_between(result.last().map(String::as_str), b)?
            };
            result.push(current);
        }
        return Ok(result);
    }
    if a.is_none() {
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            let current = if i == 0 {
                generate_key_between(a, b)?
            } else {
                generate_key_between(a, result.last().map(String::as_str))?
            };
            result.push(current);
        }
        result.reverse();
        return Ok(result);
    }

    let mid = n / 2;
    let center = generate_key_between(a, b)?;
    let mut result = generate_n_keys_between(a, Some(center.as_str()), mid)?;
    let right = generate_n_keys_between(Some(center.as_str()), b, n - mid - 1)?;
    result.push(center);
    result.extend(right);
    Ok(result)
}

/// A validated fractional-index position key. It serializes as a plain string.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Position(String);

impl Position {
    /// Validates and wraps a key, rejecting anything the generator could not
    /// have produced.
    pub fn parse(value: impl Into<String>) -> Result<Self, PositionError> {
        let value = value.into();
        validate_order_key(&value)?;
        Ok(Self(value))
    }

    /// The key as stored; comparisons are byte-wise.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Position {
    type Error = PositionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl FromStr for Position {
    type Err = PositionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl From<Position> for String {
    fn from(value: Position) -> Self {
        value.0
    }
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for Position {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Position {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Why a key could not be parsed, or why no key exists between the bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionError {
    InvalidKey(String),
    InvalidBounds { before: String, after: String },
    CannotIncrement,
    CannotDecrement,
}

impl fmt::Display for PositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey(message) => formatter.write_str(message),
            Self::InvalidBounds { before, after } => {
                write!(formatter, "invalid bounds: {before} >= {after}")
            }
            Self::CannotIncrement => formatter.write_str("cannot increment any more"),
            Self::CannotDecrement => formatter.write_str("cannot decrement any more"),
        }
    }
}

impl std::error::Error for PositionError {}

/// Returns a key strictly between `before` and `after`; `None` leaves that side
/// open (append or prepend).
pub fn key_between(
    before: Option<&Position>,
    after: Option<&Position>,
) -> Result<Position, PositionError> {
    let key = generate_key_between(before.map(Position::as_str), after.map(Position::as_str))?;
    Ok(Position(key))
}

/// Returns `n` distinct keys in ascending order between the bounds. Used for
/// backfills and bulk inserts.
pub fn n_keys_between(
    before: Option<&Position>,
    after: Option<&Position>,
    n: usize,
) -> Result<Vec<Position>, PositionError> {
    let keys =
        generate_n_keys_between(before.map(Position::as_str), after.map(Position::as_str), n)?;
    Ok(keys.into_iter().map(Position).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn position(value: &str) -> Position {
        Position::parse(value).unwrap()
    }

    fn key(a: Option<&str>, b: Option<&str>) -> Result<String, PositionError> {
        let a = a.map(position);
        let b = b.map(position);
        key_between(a.as_ref(), b.as_ref()).map(|key| key.as_str().to_string())
    }

    fn check(a: Option<&str>, b: Option<&str>, expected: &str) {
        assert_eq!(key(a, b).unwrap(), expected);
    }

    fn check_error(a: Option<&str>, b: Option<&str>, expected: &str) {
        assert_eq!(
            generate_key_between(a, b).unwrap_err().to_string(),
            expected
        );
    }

    fn keys(a: Option<&str>, b: Option<&str>, n: usize) -> Result<Vec<String>, PositionError> {
        let a = a.map(position);
        let b = b.map(position);
        n_keys_between(a.as_ref(), b.as_ref(), n).map(|keys| {
            keys.into_iter()
                .map(|key| key.as_str().to_string())
                .collect()
        })
    }

    fn expected(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn matches_reference_default_cases() {
        check(None, None, "a0");
        check(None, Some("a0"), "Zz");
        check(None, Some("Zz"), "Zy");
        check(Some("a0"), None, "a1");
        check(Some("a1"), None, "a2");
        check(Some("a0"), Some("a1"), "a0V");
        check(Some("a1"), Some("a2"), "a1V");
        check(Some("a0V"), Some("a1"), "a0l");
        check(Some("Zz"), Some("a0"), "ZzV");
        check(Some("Zz"), Some("a1"), "a0");
        check(None, Some("Y00"), "Xzzz");
        check(Some("bzz"), None, "c000");
        check(Some("a0"), Some("a0V"), "a0G");
        check(Some("a0"), Some("a0G"), "a08");
        check(Some("b125"), Some("b129"), "b127");
        check(Some("a0"), Some("a1V"), "a1");
        check(Some("Zz"), Some("a01"), "a0");
        check(None, Some("a0V"), "a0");
        check(None, Some("b999"), "b99");
        check_error(
            None,
            Some("A00000000000000000000000000"),
            "invalid order key: A00000000000000000000000000",
        );
        check(
            None,
            Some("A000000000000000000000000001"),
            "A000000000000000000000000000V",
        );
        check(
            Some("zzzzzzzzzzzzzzzzzzzzzzzzzzy"),
            None,
            "zzzzzzzzzzzzzzzzzzzzzzzzzzz",
        );
        check(
            Some("zzzzzzzzzzzzzzzzzzzzzzzzzzz"),
            None,
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzV",
        );
        check_error(Some("a00"), None, "invalid order key: a00");
        check_error(Some("a00"), Some("a1"), "invalid order key: a00");
        check_error(Some("0"), Some("1"), "invalid order key head: 0");
        check(Some("a1"), Some("a0"), "a0V");
    }

    #[test]
    fn matches_reference_n_key_vectors() {
        assert_eq!(keys(None, None, 2).unwrap(), expected(&["a0", "a1"]));
        assert_eq!(keys(Some("a1"), None, 2).unwrap(), expected(&["a2", "a3"]));
        assert_eq!(keys(None, Some("a0"), 2).unwrap(), expected(&["Zy", "Zz"]));
        assert_eq!(
            keys(Some("a1"), Some("a2"), 2).unwrap(),
            expected(&["a1G", "a1V"])
        );
        assert_eq!(
            keys(None, None, 5).unwrap(),
            expected(&["a0", "a1", "a2", "a3", "a4"])
        );
        assert_eq!(
            keys(Some("a0"), Some("a1"), 5).unwrap(),
            expected(&["a08", "a0G", "a0V", "a0d", "a0l"])
        );
        assert_eq!(
            keys(Some("a0"), None, 5).unwrap(),
            expected(&["a1", "a2", "a3", "a4", "a5"])
        );
        assert_eq!(
            keys(None, Some("a0"), 5).unwrap(),
            expected(&["Zv", "Zw", "Zx", "Zy", "Zz"])
        );
        assert!(keys(None, None, 0).unwrap().is_empty());
    }

    #[test]
    fn narrows_gap_a_thousand_times() {
        let low = position("a0");
        let high = position("a1");
        let mut ordered = vec![low.clone(), high.clone()];

        for _ in 0..1000 {
            let next = key_between(Some(&low), Some(&ordered[1])).unwrap();
            assert!(low < next);
            assert!(next < ordered[1]);
            ordered.insert(1, next);
        }

        assert_eq!(ordered.len(), 1002);
        assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(ordered[1].as_str().len(), 169);
    }

    #[test]
    fn random_insertions_stay_sorted_and_unique() {
        let mut seed: u64 = 1;
        let mut list: Vec<Position> = Vec::new();

        for _ in 0..5000 {
            seed = (seed.wrapping_mul(1_103_515_245).wrapping_add(12_345)) & 0x7fff_ffff;
            let gap = (seed as u128 * (list.len() as u128 + 1) / 0x7fff_ffffu128) as usize;
            let gap = gap.min(list.len());

            let before = if gap > 0 { Some(&list[gap - 1]) } else { None };
            let after = if gap < list.len() {
                Some(&list[gap])
            } else {
                None
            };
            let next = key_between(before, after).unwrap();

            if let Some(before) = before {
                assert!(before < &next);
            }
            if let Some(after) = after {
                assert!(&next < after);
            }

            list.insert(gap, next);
        }

        assert!(list.windows(2).all(|pair| pair[0] < pair[1]));
        let unique: HashSet<_> = list.iter().collect();
        assert_eq!(unique.len(), list.len());
    }

    #[test]
    fn validates_keys() {
        assert!(Position::parse("").is_err());
        assert!(Position::parse("0").is_err());
        assert!(Position::parse("a0").is_ok());
        assert!(Position::parse("a00").is_err());
        assert!(Position::parse("hello!").is_err());
        assert!(Position::parse("Zz").is_ok());
        assert!(Position::parse("a1V").is_ok());
        assert!(Position::parse("A00000000000000000000000000").is_err());
    }

    #[test]
    fn generated_keys_are_stable() {
        let a = position("a0");
        let b = position("a1");

        let first = key_between(Some(&a), Some(&b)).unwrap();
        let second = key_between(Some(&a), Some(&b)).unwrap();
        assert_eq!(first, second);

        let lower = key_between(Some(&a), Some(&first)).unwrap();
        let upper = key_between(Some(&first), Some(&b)).unwrap();

        // removing the key and reinserting it between the same neighbours
        // produces it again, on both sides
        assert_eq!(key_between(Some(&a), Some(&b)).unwrap(), first);
        assert_eq!(key_between(Some(&a), Some(&first)).unwrap(), lower);
        assert_eq!(key_between(Some(&first), Some(&b)).unwrap(), upper);
    }
}
