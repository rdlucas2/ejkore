/// Fixed-point number with 16 fractional bits.
/// Guarantees deterministic arithmetic across platforms (no floating-point).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fp(i32);

const FRAC_BITS: i32 = 16;
const SCALE: i32 = 1 << FRAC_BITS; // 65536

impl Fp {
    pub const ZERO: Fp = Fp(0);
    pub const ONE: Fp = Fp(SCALE);

    pub const fn from_int(n: i32) -> Fp {
        Fp(n << FRAC_BITS)
    }

    pub const fn from_raw(raw: i32) -> Fp {
        Fp(raw)
    }

    pub const fn raw(self) -> i32 {
        self.0
    }

    pub const fn to_int(self) -> i32 {
        self.0 >> FRAC_BITS
    }
}

impl core::ops::Add for Fp {
    type Output = Fp;
    fn add(self, rhs: Fp) -> Fp {
        Fp(self.0 + rhs.0)
    }
}

impl core::ops::Sub for Fp {
    type Output = Fp;
    fn sub(self, rhs: Fp) -> Fp {
        Fp(self.0 - rhs.0)
    }
}

impl core::ops::Mul for Fp {
    type Output = Fp;
    fn mul(self, rhs: Fp) -> Fp {
        // Use i64 intermediate to avoid overflow
        let result = (self.0 as i64 * rhs.0 as i64) >> FRAC_BITS;
        Fp(result as i32)
    }
}

impl core::ops::Div for Fp {
    type Output = Fp;
    fn div(self, rhs: Fp) -> Fp {
        // Use i64 intermediate to maintain precision
        let result = ((self.0 as i64) << FRAC_BITS) / rhs.0 as i64;
        Fp(result as i32)
    }
}

impl core::ops::Neg for Fp {
    type Output = Fp;
    fn neg(self) -> Fp {
        Fp(-self.0)
    }
}

impl core::ops::AddAssign for Fp {
    fn add_assign(&mut self, rhs: Fp) {
        self.0 += rhs.0;
    }
}

impl core::ops::SubAssign for Fp {
    fn sub_assign(&mut self, rhs: Fp) {
        self.0 -= rhs.0;
    }
}
