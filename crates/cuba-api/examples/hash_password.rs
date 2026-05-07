use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let password = if args.len() > 1 {
        &args[1]
    } else {
        "Admin@123456"
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    println!("Password: {}", password);
    println!("Hash: {}", password_hash);
}
