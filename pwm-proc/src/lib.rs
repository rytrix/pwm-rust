extern crate getrandom;
extern crate proc_macro;
use std::{env, io::Read, str::FromStr};

use getrandom::getrandom;
use proc_macro::TokenStream;

static PEPPER_PATH: &str = "private/pepper";

#[proc_macro]
pub fn random_pepper(_input: TokenStream) -> TokenStream {
    let mut random: [u8; 32] = [0; 32];

    let current_dir = env::current_dir().unwrap();
    let full_path = current_dir.join(PEPPER_PATH);

    match std::fs::metadata(full_path.clone()) {
        Ok(_) => {
            let mut file = std::fs::File::open(PEPPER_PATH).unwrap();
            file.read(&mut random).unwrap();
        }
        Err(_error) => {
            getrandom(&mut random).unwrap();
            std::fs::write(full_path.to_str().unwrap(), &random).unwrap();
        }
    };

    let mut stream = "[".to_string();
    for num in random {
        stream += format!("{},", num).as_str();
    }
    stream += "]";

    return TokenStream::from_str(&stream).unwrap();
}
