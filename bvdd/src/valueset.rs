use std::fmt;
use std::ops;

/// A 256-bit bitmask representing a set of byte values [0, 255].
/// Bit i is set iff value i is in the set.
///
/// Operations map directly to solving steps:
/// - AND = propagation (intersect feasible values)
/// - OR  = resolution (merge edges to same child)
/// - NOT = complement
/// - popcount = 0 -> conflict, = 1 -> unit, = 256 -> unconstrained
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueSet {
    pub bits: [u64; 4],
}

impl ValueSet {
    pub const EMPTY: ValueSet = ValueSet { bits: [0; 4] };
    pub const FULL: ValueSet = ValueSet { bits: [u64::MAX; 4] };

    /// Create a singleton set containing only value `v`.
    pub fn singleton(v: u8) -> ValueSet {
        let mut vs = ValueSet::EMPTY;
        let word = (v / 64) as usize;
        let bit = v % 64;
        vs.bits[word] = 1u64 << bit;
        vs
    }

    /// Create a set from a range [lo, hi] inclusive.
    pub fn from_range(lo: u8, hi: u8) -> ValueSet {
        let mut vs = ValueSet::EMPTY;
        for v in lo..=hi {
            vs = vs.or(ValueSet::singleton(v));
        }
        vs
    }

    /// Intersection (propagation)
    pub fn and(self, other: ValueSet) -> ValueSet {
        ValueSet {
            bits: [
                self.bits[0] & other.bits[0],
                self.bits[1] & other.bits[1],
                self.bits[2] & other.bits[2],
                self.bits[3] & other.bits[3],
            ],
        }
    }

    /// Union (resolution)
    pub fn or(self, other: ValueSet) -> ValueSet {
        ValueSet {
            bits: [
                self.bits[0] | other.bits[0],
                self.bits[1] | other.bits[1],
                self.bits[2] | other.bits[2],
                self.bits[3] | other.bits[3],
            ],
        }
    }

    /// Complement
    pub fn complement(self) -> ValueSet {
        !self
    }

    /// Number of values in the set
    pub fn popcount(self) -> u32 {
        self.bits.iter().map(|w| w.count_ones()).sum()
    }

    pub fn is_empty(self) -> bool {
        self.bits == [0; 4]
    }

    pub fn is_full(self) -> bool {
        self.bits == [u64::MAX; 4]
    }

    /// Check if value `v` is in the set
    pub fn contains(self, v: u8) -> bool {
        let word = (v / 64) as usize;
        let bit = v % 64;
        (self.bits[word] >> bit) & 1 == 1
    }

    /// Check if self is a subset of other
    pub fn is_subset_of(self, other: ValueSet) -> bool {
        self.and(other) == self
    }

    /// For a width < 8, mask to only valid domain values [0, 2^width - 1]
    pub fn mask_to_width(self, width: u16) -> ValueSet {
        if width >= 8 {
            return self;
        }
        let domain_size = 1u64 << width;
        // Only the first `domain_size` bits are valid
        ValueSet {
            bits: [self.bits[0] & ((1u64 << domain_size) - 1), 0, 0, 0],
        }
    }

    /// Full set for a given width: all values [0, 2^width - 1]
    pub fn full_for_width(width: u16) -> ValueSet {
        if width >= 8 {
            return ValueSet::FULL;
        }
        let domain_size = 1u64 << width;
        ValueSet {
            bits: [(1u64 << domain_size) - 1, 0, 0, 0],
        }
    }
}

impl ops::Not for ValueSet {
    type Output = ValueSet;
    fn not(self) -> ValueSet {
        ValueSet {
            bits: [
                !self.bits[0],
                !self.bits[1],
                !self.bits[2],
                !self.bits[3],
            ],
        }
    }
}

impl fmt::Debug for ValueSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.popcount();
        if count == 0 {
            write!(f, "empty")
        } else if count == 256 {
            write!(f, "[0..255]")
        } else if count <= 8 {
            let mut vals = Vec::new();
            for i in 0..=255u8 {
                if self.contains(i) {
                    vals.push(i.to_string());
                }
            }
            write!(f, "{{{}}}", vals.join(", "))
        } else {
            write!(f, "ValueSet({})", count)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singleton() {
        let s = ValueSet::singleton(42);
        assert!(s.contains(42));
        assert!(!s.contains(41));
        assert_eq!(s.popcount(), 1);
    }

    #[test]
    fn test_and_or_not() {
        let a = ValueSet::singleton(1).or(ValueSet::singleton(2));
        let b = ValueSet::singleton(2).or(ValueSet::singleton(3));
        let inter = a.and(b);
        assert_eq!(inter, ValueSet::singleton(2));
        let union = a.or(b);
        assert_eq!(union.popcount(), 3);
    }

    #[test]
    fn test_full_empty() {
        assert!(ValueSet::EMPTY.is_empty());
        assert!(ValueSet::FULL.is_full());
        assert_eq!(ValueSet::FULL.popcount(), 256);
        assert_eq!(ValueSet::FULL.and(ValueSet::EMPTY), ValueSet::EMPTY);
    }

    #[test]
    fn test_subset() {
        let a = ValueSet::singleton(5);
        let b = ValueSet::singleton(5).or(ValueSet::singleton(10));
        assert!(a.is_subset_of(b));
        assert!(!b.is_subset_of(a));
    }

    #[test]
    fn test_width_masking() {
        let full = ValueSet::full_for_width(2);
        assert_eq!(full.popcount(), 4); // {0, 1, 2, 3}
        assert!(full.contains(0));
        assert!(full.contains(3));
        assert!(!full.contains(4));
    }
}
