mod aes_wrapper;
mod argon2_wrapper;
mod pbkdf2_wrapper;
mod salt;
mod scrypt_wrapper;

// use scrypt_wrapper::scrypt_hash_password;
// use argon2_wrapper::argon2_hash_password;
use pbkdf2_wrapper::pbkdf2_hash_password;

use aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt};
use zeroize::Zeroizing;

fn main() {
    let password = b"hunter42"; // Bad password; don't actually use!
    let plaintext = Zeroizing::new(*b"hello world");

    let hash = pbkdf2_hash_password(password).unwrap();

    let ciphertext = aes_gcm_encrypt(hash.get_hash(), plaintext.as_ref()).unwrap();

    let plaintext = aes_gcm_decrypt(hash.get_hash(), ciphertext.as_ref()).unwrap();

    println!("{}", String::from_utf8(plaintext.as_ref().to_vec()).unwrap());
}
