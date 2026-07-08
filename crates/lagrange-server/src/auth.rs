//! Local authentication: argon2-hashed admin accounts + stateless JWT.
//!
//! This is deliberately self-contained so `lagrange-server` has no hard
//! dependency on the heavier `kirino` RBAC framework. The account store is a
//! tiny sqlite table next to the comment tables; a `create-admin` bootstrap
//! seeds the first moderator. JWTs carry the author identity and a `mod`
//! flag, decoded per-request into a [`Caller`].

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use lagrange_protocol::{Author, Caller, IdentityKind};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

/// JWT claims. `sub` is the account id; `mod` flags moderators.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub name: String,
    pub moderator: bool,
    pub exp: usize,
}

/// The symmetric secret used to sign JWTs. Generated on first run if absent.
pub struct AuthState {
    encode: EncodingKey,
    decode: DecodingKey,
}

impl AuthState {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encode: EncodingKey::from_secret(secret),
            decode: DecodingKey::from_secret(secret),
        }
    }

    pub fn issue(&self, account: &Account) -> Result<String, ApiError> {
        let exp = (Utc::now() + Duration::days(30)).timestamp() as usize;
        let claims = Claims {
            sub: account.id.clone(),
            name: account.name.clone(),
            moderator: account.moderator,
            exp,
        };
        encode(&Header::default(), &claims, &self.encode)
            .map_err(|e| ApiError::internal(format!("jwt encode: {e}")))
    }

    pub fn verify(&self, token: &str) -> Result<Claims, ApiError> {
        decode::<Claims>(token, &self.decode, &Validation::default())
            .map(|d| d.claims)
            .map_err(|_| ApiError::unauthorized("invalid or expired token"))
    }
}

/// A local account row.
#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub moderator: bool,
}

/// Hash a password with argon2.
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::internal(format!("argon2 hash: {e}")))
}

/// Verify a password against a stored argon2 hash.
pub fn verify_password(hash: &str, password: &str) -> Result<bool, ApiError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| ApiError::internal(format!("argon2 parse: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Turn JWT claims into a protocol [`Caller`]. Non-moderator authenticated
/// users map to [`Caller::Authenticated`]; moderators to [`Caller::Moderator`].
pub fn claims_to_caller(claims: &Claims) -> Caller {
    let author = Author {
        id: Some(claims.sub.clone()),
        name: claims.name.clone(),
        avatar: None,
        identity_kind: IdentityKind::Local,
        external_id: None,
    };
    if claims.moderator {
        Caller::Moderator(author)
    } else {
        Caller::Authenticated(author)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_round_trips() {
        let hash = hash_password("hunter2").unwrap();
        assert!(verify_password(&hash, "hunter2").unwrap());
        assert!(!verify_password(&hash, "wrong").unwrap());
    }

    #[test]
    fn jwt_round_trips() {
        let state = AuthState::new(b"test-secret");
        let account = Account {
            id: "u1".into(),
            name: "Alice".into(),
            moderator: true,
        };
        let token = state.issue(&account).unwrap();
        let claims = state.verify(&token).unwrap();
        assert_eq!(claims.sub, "u1");
        assert!(claims.moderator);
        let caller = claims_to_caller(&claims);
        assert!(caller.is_moderator());
    }

    #[test]
    fn bad_token_rejected() {
        let state = AuthState::new(b"test-secret");
        assert!(state.verify("garbage").is_err());
    }

    #[test]
    fn different_secrets_reject_each_other() {
        let issuer = AuthState::new(b"secret-a");
        let verifier = AuthState::new(b"secret-b");
        let token = issuer
            .issue(&Account {
                id: "u".into(),
                name: "U".into(),
                moderator: false,
            })
            .unwrap();
        assert!(verifier.verify(&token).is_err());
    }
}
