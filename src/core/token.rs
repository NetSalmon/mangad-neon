use crate::error::Error;
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::RngExt;
use std::iter;

const BASE58_CHARS: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub fn gen_token() -> String {
    let mut rng = rand::rng();
    let ss: String =
        iter::repeat_with(|| BASE58_CHARS[rng.random_range(0..BASE58_CHARS.len())] as char)
            .take(64)
            .collect();
    format!("mangad_{}:{}", uuid::Uuid::new_v4(), ss)
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

pub fn verify(token: &str, hash: &str) -> Result<bool, Error> {
    let parse_hash = PasswordHash::new(hash).map_err(|e| Error::TokenHashError(e.to_string()))?;

    let ok = Argon2::default()
        .verify_password(token.as_bytes(), &parse_hash)
        .is_ok();

    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        let s = gen_token();
        println!("{}", s);
        let h = hash(&s).unwrap();
        println!("{}", h);
        let ok = verify(&s, &h).unwrap();
        println!("{}", ok);
    }
}
