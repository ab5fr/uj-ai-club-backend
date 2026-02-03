use bcrypt::{DEFAULT_COST, hash};
use std::io::{self, Write};

fn read_password_from_stdin() -> String {
    print!("Enter password: ");
    io::stdout().flush().expect("failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read password");

    input.trim_end_matches(&['\r', '\n'][..]).to_string()
}

fn main() {
    let password = std::env::args()
        .nth(1)
        .unwrap_or_else(read_password_from_stdin);

    let password_hash = hash(password, DEFAULT_COST).expect("failed to hash password");
    println!("{password_hash}");
}
