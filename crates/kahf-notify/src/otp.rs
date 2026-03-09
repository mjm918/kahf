//! OTP generation and TTL constants.
//!
//! ## generate_otp
//!
//! Generates a cryptographically random 6-digit numeric OTP string using
//! the `rand` crate's thread-local CSPRNG.
//!
//! ## OTP_TTL_MINUTES
//!
//! OTP expiration time in minutes (10).
//!
//! ## INVITE_TTL_DAYS
//!
//! Invitation expiration time in days (7).

use rand::Rng;

pub const OTP_TTL_MINUTES: i64 = 10;
pub const INVITE_TTL_DAYS: i64 = 7;

pub fn generate_otp() -> String {
    let mut rng = rand::rng();
    let code: u32 = rng.random_range(100_000..1_000_000);
    code.to_string()
}
