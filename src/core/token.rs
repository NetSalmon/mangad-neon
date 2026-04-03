use crate::core::entities::orm::tokens;
use crate::error::Error;
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::RngExt;
use std::iter;
use std::str::FromStr;
use uuid::Uuid;

const BASE58_CHARS: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub fn gen_token() -> (String, Uuid) {
    let mut rng = rand::rng();
    let ss: String =
        iter::repeat_with(|| BASE58_CHARS[rng.random_range(0..BASE58_CHARS.len())] as char)
            .take(64)
            .collect();
    let uuid = Uuid::new_v4();
    let token = format!("mangad_{}:{}", uuid, ss);
    (token, uuid)
}

pub fn hash(token: &str) -> Result<String, Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(token.as_bytes(), &salt)
        .map_err(|e| Error::TokenHashError(e.to_string()))?
        .to_string();
    Ok(hash)
}

pub fn verify_hash(token: &str, hash: &str) -> Result<bool, Error> {
    let parse_hash = PasswordHash::new(hash).map_err(|e| Error::TokenHashError(e.to_string()))?;

    let ok = Argon2::default()
        .verify_password(token.as_bytes(), &parse_hash)
        .is_ok();

    Ok(ok)
}

pub fn get_uuid(token: &str) -> Result<Uuid, Error> {
    let parts: Vec<&str> = token.split(':').collect();
    let uuid_part = parts
        .get(0)
        .and_then(|s| s.strip_prefix("mangad_"))
        .ok_or(Error::InvalidTokenFormatError)?;

    Ok(Uuid::from_str(uuid_part)?)
}

pub trait TokenTrait {
    fn uuid(&self) -> Result<Uuid, Error>;
}

impl TokenTrait for String {
    fn uuid(&self) -> Result<Uuid, Error> {
        get_uuid(self)
    }
}

impl TokenTrait for str {
    fn uuid(&self) -> Result<Uuid, Error> {
        get_uuid(self)
    }
}

impl TokenTrait for tokens::Model {
    fn uuid(&self) -> Result<Uuid, Error> {
        Ok(self.id)
    }
}

impl TokenTrait for Uuid {
    fn uuid(&self) -> Result<Uuid, Error> {
        Ok(*self)
    }
}
