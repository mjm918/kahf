//! Argon2id password hashing and verification.
//!
//! ## hash_password
//!
//! Hashes a plaintext password using Argon2id with a random salt.
//! Returns a PHC-format string suitable for storing in the database.
//! Uses default Argon2id parameters (memory-hard, side-channel resistant).
//!
//! ## verify_password
//!
//! Verifies a plaintext password against a stored PHC-format hash.
//! Returns `Ok(())` on match, `Err` on mismatch or invalid hash format.
//! Hash parameters are read from the stored hash, not from the Argon2
//! instance, so upgrades to stronger parameters are automatic.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

pub fn hash_password(password: &str) -> eyre::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| eyre::eyre!("failed to hash password: {e}"))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> eyre::Result<()> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| eyre::eyre!("invalid password hash format: {e}"))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| kahf_core::KahfError::unauthorized())
}
