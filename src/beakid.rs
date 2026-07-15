const TIMESTAMP_SHIFT: u8 = 22;
const SEQUENCE_SHIFT: u8 = 10;
const TIMESTAMP_MASK: i64 = (1 << 41) - 1;
const SEQUENCE_MASK: i64 = (1 << 12) - 1;
const WORKER_ID_MASK: i64 = (1 << 10) - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BeakId(i64);

impl BeakId {
    pub(crate) fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> i64 {
        self.0
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
