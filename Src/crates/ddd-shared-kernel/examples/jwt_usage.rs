use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AdminClaims {
    sub: String,
    exp: i64,
    role: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let secret = b"my-shared-secret";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    // 1. Standard usage with HS256
    let standard_validator: JwtValidator<StandardClaims> = JwtValidator::hs256(secret)
        .with_issuer(["https://auth.example.com"])
        .with_audience(["admin-bff"])
        .with_leeway(30);

    let standard_claims = StandardClaims {
        sub: "user-123".into(),
        exp: now + 3600,
        iat: Some(now),
        nbf: None,
        iss: Some("https://auth.example.com".into()),
        aud: Some("admin-bff".into()),
        scope: Some("orders.read orders.write".into()),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &standard_claims,
        &EncodingKey::from_secret(secret),
    )?;

    println!("Validating standard token...");
    let data = standard_validator.validate(&token)?;
    println!("Validated sub: {}", data.claims.sub);

    // 2. Custom claims usage
    let custom_validator: JwtValidator<AdminClaims> = JwtValidator::hs256(secret);
    let custom_claims = AdminClaims {
        sub: "admin-456".into(),
        exp: now + 3600,
        role: "super-admin".into(),
    };

    let custom_token = encode(
        &Header::new(Algorithm::HS256),
        &custom_claims,
        &EncodingKey::from_secret(secret),
    )?;

    println!("\nValidating custom token...");
    let custom_data = custom_validator.validate(&custom_token)?;
    println!("Validated role: {}", custom_data.claims.role);

    Ok(())
}
