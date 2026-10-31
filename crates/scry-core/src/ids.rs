use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

pub const MIN_SHORT_ID_LENGTH: usize = 4;
pub const MAX_SHORT_ID_LENGTH: usize = 32;

pub type ProjectId = Uuid;
pub type SectionId = Uuid;
pub type TaskId = Uuid;
pub type NoteId = Uuid;

const CANONICAL_DASH_POSITIONS: [usize; 4] = [8, 12, 16, 20];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShortId(String);

impl ShortId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    // a ShortId always contains at least MIN_SHORT_ID_LENGTH hex characters
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn range(&self) -> (Uuid, Uuid) {
        let low = format!(
            "{}{}",
            self.0,
            "0".repeat(MAX_SHORT_ID_LENGTH - self.0.len())
        );
        let high = format!(
            "{}{}",
            self.0,
            "f".repeat(MAX_SHORT_ID_LENGTH - self.0.len())
        );

        (
            Uuid::parse_str(&low).expect("a normalized short id is a valid low bound"),
            Uuid::parse_str(&high).expect("a normalized short id is a valid high bound"),
        )
    }
}

impl fmt::Display for ShortId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ShortId {
    type Err = ShortIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ShortIdError::Empty);
        }

        let mut hex = String::with_capacity(trimmed.len());
        for (index, character) in trimmed.char_indices() {
            let character = character.to_ascii_lowercase();
            if character == '-' {
                let trailing = index + 1 == trimmed.len();
                if trailing || !CANONICAL_DASH_POSITIONS.contains(&hex.len()) {
                    return Err(ShortIdError::MisplacedDash { index });
                }
            } else if character.is_ascii_hexdigit() {
                hex.push(character);
            } else {
                return Err(ShortIdError::InvalidCharacter { character, index });
            }
        }

        let length = hex.len();
        if length < MIN_SHORT_ID_LENGTH {
            return Err(ShortIdError::TooShort { length });
        }
        if length > MAX_SHORT_ID_LENGTH {
            return Err(ShortIdError::TooLong { length });
        }

        Ok(Self(hex))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortIdError {
    Empty,
    TooShort { length: usize },
    TooLong { length: usize },
    MisplacedDash { index: usize },
    InvalidCharacter { character: char, index: usize },
}

impl fmt::Display for ShortIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("a short id cannot be empty"),
            Self::TooShort { length } => write!(
                f,
                "a short id must contain at least {MIN_SHORT_ID_LENGTH} hex characters (found {length})"
            ),
            Self::TooLong { length } => write!(
                f,
                "a short id must contain at most {MAX_SHORT_ID_LENGTH} hex characters (found {length})"
            ),
            Self::MisplacedDash { index } => write!(
                f,
                "a dash is only allowed after 8, 12, 16, or 20 hex characters (found one at index {index})"
            ),
            Self::InvalidCharacter { character, index } => write!(
                f,
                "a short id may only contain hex characters and dashes, but found {character:?} at index {index}"
            ),
        }
    }
}

impl std::error::Error for ShortIdError {}

pub fn shortest_handles(
    ids: impl IntoIterator<Item = Uuid>,
    min_length: usize,
) -> HashMap<Uuid, String> {
    let min_length = min_length.clamp(MIN_SHORT_ID_LENGTH, MAX_SHORT_ID_LENGTH);

    let mut entries: Vec<(String, Uuid)> = ids
        .into_iter()
        .map(|id| (id.simple().to_string(), id))
        .collect();
    entries.sort_unstable();
    // identical UUIDs have identical hex and therefore sort next to each other
    entries.dedup_by_key(|(_, id)| *id);

    let mut handles = HashMap::with_capacity(entries.len());
    for (index, (hex, id)) in entries.iter().enumerate() {
        let previous = index
            .checked_sub(1)
            .map(|previous| longest_common_prefix(&entries[previous].0, hex));
        let next = entries
            .get(index + 1)
            .map(|(next_hex, _)| longest_common_prefix(hex, next_hex));
        let shared = previous.into_iter().chain(next).max().unwrap_or(0);
        let length = (shared + 1).clamp(min_length, MAX_SHORT_ID_LENGTH);

        handles.insert(*id, hex[..length].to_string());
    }

    handles
}

pub fn shortest_handle(id: Uuid, ids: impl IntoIterator<Item = Uuid>, min_length: usize) -> String {
    shortest_handles(ids, min_length)
        .remove(&id)
        .unwrap_or_else(|| id.simple().to_string())
}

pub fn distinguishing_prefixes(ids: &[Uuid], min_length: usize) -> Vec<String> {
    let mut prefixes: Vec<String> = shortest_handles(ids.iter().copied(), min_length)
        .into_values()
        .collect();
    prefixes.sort();

    prefixes
}

/// Namespace for scry-owned deterministic ids that are not scoped to a project.
///
/// The built-in default project is `UUIDv5(scry_namespace(), "default")`; that
/// value is pinned in the init migration. Template sections use the project UUID
/// as their namespace instead.
pub fn scry_namespace() -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"scry")
}

/// Deterministic id for a section created from a project template.
///
/// The project UUID is the namespace, so the name only has to be unique within
/// that project, matching the section-name uniqueness rule. Re-applying the same
/// template to the same project therefore reuses the same section ids.
pub fn template_section_id(project_id: ProjectId, name: &str) -> SectionId {
    Uuid::new_v5(&project_id, name.trim().to_lowercase().as_bytes())
}

fn longest_common_prefix(left: &str, right: &str) -> usize {
    left.bytes()
        .zip(right.bytes())
        .take_while(|(left, right)| left == right)
        .count()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};
    use std::str::FromStr;

    use super::*;

    const SAMPLE_UUID: &str = "4f3a2b1c-9d2e-4f5a-8b6c-1d2e3f4a5b6c";
    const SAMPLE_HEX: &str = "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6c";

    fn uuid_from_hex(hex: &str) -> Uuid {
        Uuid::parse_str(hex).unwrap()
    }

    fn lenient_handles(ids: &[Uuid], min_length: usize) -> HashMap<Uuid, String> {
        let min_length = min_length.clamp(MIN_SHORT_ID_LENGTH, MAX_SHORT_ID_LENGTH);
        let unique: BTreeSet<Uuid> = ids.iter().copied().collect();
        let entries: Vec<(Uuid, String)> = unique
            .iter()
            .map(|id| (*id, id.simple().to_string()))
            .collect();

        entries
            .iter()
            .map(|(id, hex)| {
                let shared = entries
                    .iter()
                    .filter(|entry| entry.0 != *id)
                    .map(|entry| longest_common_prefix(hex, &entry.1))
                    .max()
                    .unwrap_or(0);
                let length = (shared + 1).clamp(min_length, MAX_SHORT_ID_LENGTH);

                (*id, hex[..length].to_string())
            })
            .collect()
    }

    #[test]
    fn uuid_aliases_are_plain_uuids() {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();

        assert_eq!(first.get_version_num(), 4);
        assert_eq!(second.get_version_num(), 4);
        assert_ne!(first, second);

        let id: TaskId = Uuid::parse_str(SAMPLE_UUID).unwrap();
        assert_eq!(id.to_string(), SAMPLE_UUID);
    }

    #[test]
    fn short_id_normalizes_all_accepted_forms() {
        for input in [
            SAMPLE_HEX,
            "4F3A2B1C9D2E4F5A8B6C1D2E3F4A5B6C",
            SAMPLE_UUID,
            "4F3A2B1C-9D2E-4F5A-8B6C-1D2E3F4A5B6C",
        ] {
            assert_eq!(ShortId::from_str(input).unwrap().as_str(), SAMPLE_HEX);
        }
    }

    #[test]
    fn short_id_rejects_invalid_input() {
        assert_eq!(
            ShortId::from_str("abc"),
            Err(ShortIdError::TooShort { length: 3 })
        );
        assert_eq!(
            ShortId::from_str(&"0".repeat(33)),
            Err(ShortIdError::TooLong { length: 33 })
        );
        assert_eq!(
            ShortId::from_str("zzzz"),
            Err(ShortIdError::InvalidCharacter {
                character: 'z',
                index: 0
            })
        );
        assert_eq!(
            ShortId::from_str("4f3a-2b1c"),
            Err(ShortIdError::MisplacedDash { index: 4 })
        );
        assert_eq!(
            ShortId::from_str("4f3a2b1c-"),
            Err(ShortIdError::MisplacedDash { index: 8 })
        );
    }

    #[test]
    fn short_id_accepts_only_canonical_dash_positions() {
        for input in [
            "4f3a2b1c-9d2e",
            "4f3a2b1c9d2e-4f5a",
            "4f3a2b1c9d2e4f5a-8b6c",
            "4f3a2b1c9d2e4f5a8b6c-1d2e",
        ] {
            assert!(ShortId::from_str(input).is_ok(), "{input} should be valid");
        }

        for input in ["4f3a-2b1c", "4f3a2b1c9-d2e", "-4f3a2b1c", "4f3a2b1c-"] {
            assert!(
                ShortId::from_str(input).is_err(),
                "{input} should be invalid"
            );
        }
    }

    #[test]
    fn short_id_range_pads_low_and_high_bounds() {
        for (prefix, padding) in [("4f3a", 28), ("4f3a2", 27), ("4f3a2b", 26)] {
            let short = ShortId::from_str(prefix).unwrap();
            let (low, high) = short.range();

            assert_eq!(
                low,
                Uuid::parse_str(&format!("{prefix}{}", "0".repeat(padding))).unwrap()
            );
            assert_eq!(
                high,
                Uuid::parse_str(&format!("{prefix}{}", "f".repeat(padding))).unwrap()
            );
            assert!(low < high);
        }

        let short = ShortId::from_str("ffff").unwrap();
        let (low, high) = short.range();
        assert_eq!(
            low,
            Uuid::parse_str(&format!("ffff{}", "0".repeat(28))).unwrap()
        );
        assert_eq!(high, Uuid::parse_str(&"f".repeat(32)).unwrap());

        let short = ShortId::from_str(SAMPLE_HEX).unwrap();
        let (low, high) = short.range();
        assert_eq!(low, uuid_from_hex(SAMPLE_HEX));
        assert_eq!(high, uuid_from_hex(SAMPLE_HEX));
    }

    #[test]
    fn shortest_handles_match_brute_force() {
        let hexes = [
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6c",
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6d",
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b7a",
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5c00",
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4a6000",
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4b0000",
            "4f3a2b1c9d2e4f5a8b6c1d2e3f500000",
            "4f3a2b1c9d2e4f5a8b6c1d2e3f400000",
        ];
        let ids: Vec<Uuid> = hexes.iter().map(|hex| uuid_from_hex(hex)).collect();

        for min_length in [0, MIN_SHORT_ID_LENGTH, 8, 16] {
            assert_eq!(
                shortest_handles(ids.iter().copied(), min_length),
                lenient_handles(&ids, min_length),
                "min_length {min_length}"
            );
        }

        let single = [ids[0]];
        assert_eq!(
            shortest_handles(single.iter().copied(), MIN_SHORT_ID_LENGTH),
            lenient_handles(&single, MIN_SHORT_ID_LENGTH)
        );
        assert_eq!(
            shortest_handles(single.iter().copied(), MIN_SHORT_ID_LENGTH)[&ids[0]].len(),
            MIN_SHORT_ID_LENGTH
        );
    }

    #[test]
    fn adding_a_sharing_id_lengthens_the_handle() {
        let first = uuid_from_hex("4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6c");
        let second = uuid_from_hex("4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6d");

        let alone = shortest_handle(first, [first], MIN_SHORT_ID_LENGTH);
        let shared = shortest_handle(first, [first, second], MIN_SHORT_ID_LENGTH);

        assert_eq!(alone.len(), MIN_SHORT_ID_LENGTH);
        assert!(shared.len() > alone.len());
    }

    #[test]
    fn min_length_only_lengthens_handles_and_is_clamped() {
        let id = uuid_from_hex("4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6c");
        let ids = [id];

        let base = shortest_handles(ids, MIN_SHORT_ID_LENGTH);
        let raised = shortest_handles(ids, 8);
        let clamped = shortest_handles(ids, 0);
        let capped = shortest_handles(ids, 1_000);

        assert!(raised[&id].len() > base[&id].len());
        assert!(raised[&id].len() >= 8);
        assert_eq!(clamped[&id], base[&id]);
        assert!(clamped[&id].len() >= MIN_SHORT_ID_LENGTH);
        assert_eq!(capped[&id].len(), MAX_SHORT_ID_LENGTH);
    }

    #[test]
    fn distinguishing_prefixes_separate_every_candidate() {
        let ids: Vec<Uuid> = [
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6c",
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6d",
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b7a",
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5c00",
        ]
        .iter()
        .map(|hex| uuid_from_hex(hex))
        .collect();

        let prefixes = distinguishing_prefixes(&ids, MIN_SHORT_ID_LENGTH);
        let unique: BTreeSet<&String> = prefixes.iter().collect();

        assert_eq!(prefixes.len(), ids.len());
        assert_eq!(unique.len(), ids.len());
        assert!(prefixes.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn scry_namespace_and_default_project_uuid_are_stable() {
        assert_eq!(
            scry_namespace(),
            Uuid::parse_str("20a09144-7586-58ed-ad3d-6080631a1b10").unwrap()
        );
        // keep the SQL seed in migrations/20261031164943_init.up.sql in sync
        assert_eq!(
            Uuid::parse_str("b4bf48f3-2fdb-5b9f-b55f-2f4ffc38bb08").unwrap(),
            Uuid::new_v5(&scry_namespace(), b"default")
        );
    }

    #[test]
    fn template_section_ids_are_deterministic_per_project() {
        let project = uuid_from_hex(SAMPLE_HEX);
        let other = uuid_from_hex("9d2e4f5a8b6c1d2e3f4a5b6c4f3a2b1c");

        let first = template_section_id(project, "In Progress");
        let second = template_section_id(project, "  in progress  ");

        assert_eq!(first, second);
        assert_eq!(first, Uuid::new_v5(&project, b"in progress"));
        assert_ne!(first, template_section_id(other, "In Progress"));
        assert_ne!(first, template_section_id(project, "Review"));
    }
}
