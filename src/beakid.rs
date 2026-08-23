#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const TIMESTAMP_SHIFT: u8 = 22;
const SEQUENCE_SHIFT: u8 = 10;
const TIMESTAMP_MASK: i64 = (1 << 41) - 1;
const SEQUENCE_MASK: i64 = (1 << 12) - 1;
const WORKER_ID_MASK: i64 = (1 << 10) - 1;
const BASE62_ALPHABET: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BeakId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base62ParseError {
    Empty,
    InvalidCharacter(char),
    Overflow,
}

impl std::fmt::Display for Base62ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "base62 string is empty"),
            Self::InvalidCharacter(ch) => write!(f, "invalid base62 character: {ch}"),
            Self::Overflow => write!(f, "base62 value overflows i64"),
        }
    }
}

impl std::error::Error for Base62ParseError {}

impl BeakId {
    pub(crate) fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> i64 {
        self.0
    }

    pub const fn to_u64(self) -> u64 {
        self.0 as u64
    }

    pub const fn from_u64(value: u64) -> Self {
        Self(value as i64)
    }

    pub const fn timestamp(self) -> i64 {
        (self.0 >> TIMESTAMP_SHIFT) & TIMESTAMP_MASK
    }

    pub const fn sequence(self) -> u16 {
        ((self.0 >> SEQUENCE_SHIFT) & SEQUENCE_MASK) as u16
    }

    pub const fn worker_id(self) -> u16 {
        (self.0 & WORKER_ID_MASK) as u16
    }

    pub fn to_base62(self) -> String {
        if self.0 == 0 {
            return "0".to_string();
        }

        let mut value = self.0 as u64;
        let mut encoded = [0u8; 11];
        let mut index = encoded.len();

        while value > 0 {
            index -= 1;
            encoded[index] = BASE62_ALPHABET[(value % 62) as usize];
            value /= 62;
        }

        String::from_utf8(encoded[index..].to_vec()).expect("base62 alphabet is valid UTF-8")
    }

    pub fn base62(self) -> String {
        self.to_base62()
    }

    pub fn from_base62(value: &str) -> Result<Self, Base62ParseError> {
        if value.is_empty() {
            return Err(Base62ParseError::Empty);
        }

        let mut raw = 0i64;
        for ch in value.chars() {
            let digit = base62_digit(ch).ok_or(Base62ParseError::InvalidCharacter(ch))?;
            raw = raw
                .checked_mul(62)
                .and_then(|raw| raw.checked_add(digit))
                .ok_or(Base62ParseError::Overflow)?;
        }

        Ok(Self(raw))
    }
}

fn base62_digit(ch: char) -> Option<i64> {
    match ch {
        '0'..='9' => Some(ch as i64 - '0' as i64),
        'A'..='Z' => Some(ch as i64 - 'A' as i64 + 10),
        'a'..='z' => Some(ch as i64 - 'a' as i64 + 36),
        _ => None,
    }
}

impl From<BeakId> for i64 {
    fn from(id: BeakId) -> i64 {
        id.0
    }
}

impl std::fmt::Display for BeakId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
