// use pwm_lib::scrypt_wrapper::scrypt_hash_password;
// use pwm_lib::argon2_wrapper::argon2_hash_password;
use pwm_lib::hash::pbkdf2_wrapper::pbkdf2_hash_password;

use pwm_lib::aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt};
use pwm_lib::zeroize::Zeroizing;

fn main() {

    let password = Zeroizing::new(*b"hunter42"); // Bad password; don't actually use!
    let plaintext = Zeroizing::new(*b"hello world");

    let hash = pbkdf2_hash_password(password.as_ref()).unwrap();

    let ciphertext = aes_gcm_encrypt(&hash, plaintext.as_ref()).unwrap();

    let plaintext = aes_gcm_decrypt(hash.get_hash(), &ciphertext).unwrap();

    println!(
        "{}",
        String::from_utf8(plaintext.as_ref().to_vec()).unwrap()
    );
}

// use rusqlite::serialize::{Data, OwnedData};
// use rusqlite::{Connection, DatabaseName};

// fn sqlite() {
//     let connection = Connection::open_in_memory().unwrap();
//     connection
//         .execute(
//             "CREATE TABLE Test (
//             name TEXT NOT NULL,
//             data INTEGER)",
//             (),
//         )
//         .unwrap();

//     connection
//         .execute(
//             "INSERT INTO Test (name, data) VALUES (?1, ?2)",
//             ("Ryan", 10),
//         )
//         .unwrap();

//     let serialized = connection.serialize(DatabaseName::Main).unwrap();

//     let serialized_ref = serialized.as_ref();

//     let Data::Owned(serialized) = serialized else {
//         panic!("expected OwnedData")
//     };

//     let mut connection = Connection::open_in_memory().unwrap();
//     connection.deserialize(DatabaseName::Main, serialized, false).unwrap();

//     #[derive(Debug)]
//     struct Test {
//         name: String,
//         data: i32,
//     }

//     let mut stmt = connection.prepare("SELECT name, data FROM Test").unwrap();
//     let data_iter = stmt.query_map([], |row| {
//         Ok(Test{name: row.get(0).unwrap(), data: row.get(1).unwrap()})
//     }).unwrap();

//     for t in data_iter {
//         println!("found {:?}", t.unwrap());
//     }
// }
