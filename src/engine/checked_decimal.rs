use std::fmt;

use rust_decimal::Decimal;
use serde::{Serialize};

use crate::error::Error;

/// A helper type for checked decimal operations to ensure error handling and prevent panic on overflow/underflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct CheckedDecimal(Decimal);

impl CheckedDecimal {
    pub const ZERO: Self = CheckedDecimal(Decimal::ZERO);
    const PRECISION: u32 = 4;

    // Creates a new `CheckedDecimal` from a string, rounding to the defined precision.
    pub fn parse(value: &str) -> Result<Self, Error> {
        Decimal::from_str_exact(value)
            .map(|v| {
                CheckedDecimal(v.round_dp(Self::PRECISION))
            })
            .map_err(Error::ParseDecimal)
    }

    pub fn is_sign_negative(self) -> bool {
        self.0.is_sign_negative()
    }

    pub fn checked_add(self, other: CheckedDecimal) -> Result<Self, Error> {
        self.0.checked_add(other.0).map(CheckedDecimal).ok_or(Error::DecimalOverflow)
    }

    pub fn checked_sub(self, other: CheckedDecimal) -> Result<Self, Error> {
        self.0.checked_sub(other.0).map(CheckedDecimal).ok_or(Error::DecimalUnderflow)
    }
}

impl TryFrom<&str> for CheckedDecimal {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl fmt::Display for CheckedDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Decimal> for CheckedDecimal {
    fn from(value: Decimal) -> Self {
        CheckedDecimal(value.round_dp(Self::PRECISION))
    }
}