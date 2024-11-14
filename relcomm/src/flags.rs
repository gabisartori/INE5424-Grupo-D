
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// ack: 1, last: 2, gsp: 4, syn: 8, fin: 16
pub struct Flags {
    pub value: u8,
}

impl Flags {
    pub const EMP: Flags = Flags { value: 0 };
    pub const ACK: Flags = Flags { value: 1 };
    pub const LST: Flags = Flags { value: 2 };
    pub const BRD: Flags = Flags { value: 4 };
    pub const HB: Flags = Flags { value: 8 };

    pub fn is_set(&self, flag: Flags) -> bool {
        self.value & flag.value != 0
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();
        if self.is_set(Flags::ACK) {
            result.push_str("ACK ");
        }
        if self.is_set(Flags::LST) {
            result.push_str("LST ");
        }
        if self.is_set(Flags::BRD) {
            result.push_str("BRD ");
        }
        if self.is_set(Flags::HB) {
            result.push_str("HB ");
        }
        result
    }
}

impl std::ops::BitOr for Flags {
    type Output = Flags;

    fn bitor(self, rhs: Self) -> Self::Output {
        Flags { value: self.value | rhs.value }
    }
}

impl std::ops::BitOr<u8> for Flags {
    type Output = Flags;

    fn bitor(self, rhs: u8) -> Self::Output {
        Flags { value: self.value | rhs }
    }
}

impl std::ops::BitAnd for Flags {
    type Output = Flags;

    fn bitand(self, rhs: Self) -> Self::Output {
        Flags { value: self.value & rhs.value }
    }
}

impl std::ops::BitAnd<u8> for Flags {
    type Output = Flags;

    fn bitand(self, rhs: u8) -> Self::Output {
        Flags { value: self.value & rhs }
    }
}

impl std::ops::BitAnd<bool> for Flags {
    type Output = Flags;

    fn bitand(self, rhs: bool) -> Self::Output {
        Flags { value: if rhs { self.value } else { 0 } }
    }
}

impl std::ops::BitAnd<Flags> for bool {
    type Output = Flags;

    fn bitand(self, rhs: Flags) -> Self::Output {
        Flags { value: if self { rhs.value } else { 0 } }
    }
}

impl std::ops::Not for Flags {
    type Output = Flags;

    fn not(self) -> Self::Output {
        Flags { value: !self.value }
    }
}

impl std::fmt::Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<u8> for Flags {
    fn from(value: u8) -> Self {
        Flags { value }
    }
}
