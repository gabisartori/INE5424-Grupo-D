
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    pub value: u8,
}

impl Flags {
    pub const EMP: Flags = Flags { value: 0 };
    pub const ACK: Flags = Flags { value: 1 };
    pub const LST: Flags = Flags { value: 2 };
    pub const SYN: Flags = Flags { value: 4 };
    pub const FIN: Flags = Flags { value: 8 };

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
        if self.is_set(Flags::SYN) {
            result.push_str("SYN ");
        }
        if self.is_set(Flags::FIN) {
            result.push_str("FIN ");
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

impl std::ops::BitAnd for Flags {
    type Output = Flags;

    fn bitand(self, rhs: Self) -> Self::Output {
        Flags { value: self.value & rhs.value }
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