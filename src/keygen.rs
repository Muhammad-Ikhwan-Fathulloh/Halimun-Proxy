use clap::Parser;
use rand::RngCore;
use serde_json::json;

#[derive(Parser, Debug)]
pub struct KeygenArgs {
    /// Format to output the generated keys (json, env, yaml)
    #[arg(short, long, default_value = "env")]
    pub format: String,
}

pub fn generate_keys(format: &str) {
    let mut aes_key_bytes = [0u8; 32];
    let mut hmac_key_bytes = [0u8; 32];

    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut aes_key_bytes);
    rng.fill_bytes(&mut hmac_key_bytes);

    let aes_hex = hex::encode(aes_key_bytes);
    let hmac_hex = hex::encode(hmac_key_bytes);

    let xor_key: u8 = 172; // 0xAC
    let base32_alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

    match format.to_lowercase().as_str() {
        "json" => {
            let output = json!({
                "HALIMUN_AES_KEY": aes_hex,
                "HALIMUN_HMAC_KEY": hmac_hex,
                "HALIMUN_XOR_KEY": xor_key,
                "HALIMUN_BASE32_ALPHABET": base32_alphabet
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        "yaml" => {
            println!("encryption:");
            println!("  aes_key: \"{}\"", aes_hex);
            println!("  hmac_key: \"{}\"", hmac_hex);
            println!("  xor_key: {}", xor_key);
            println!("  base32_alphabet: \"{}\"", base32_alphabet);
        }
        _ => {
            // default to env
            println!("HALIMUN_AES_KEY={}", aes_hex);
            println!("HALIMUN_HMAC_KEY={}", hmac_hex);
            println!("HALIMUN_XOR_KEY={}", xor_key);
            println!("HALIMUN_BASE32_ALPHABET={}", base32_alphabet);
        }
    }
}
