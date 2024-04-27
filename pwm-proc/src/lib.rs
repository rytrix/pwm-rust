extern crate getrandom;
extern crate proc_macro;
use std::{io::Read, str::FromStr};

use getrandom::getrandom;
use proc_macro::TokenStream;

static PEPPER_PATH: &str = "private/pepper";

#[proc_macro]
pub fn random_number(_input: TokenStream) -> TokenStream {
    let mut random: [u8; 32] = [0; 32];
    match std::fs::metadata(PEPPER_PATH) {
        Ok(_) => {
            let mut file = std::fs::File::open(PEPPER_PATH).unwrap();
            file.read(&mut random).unwrap();
        }
        Err(_error) => {
            getrandom(&mut random).unwrap();
            std::fs::write(PEPPER_PATH, &random).unwrap();
        }
    };

    let mut stream = "[".to_string();
    for num in random {
        stream += format!("{},", num).as_str();
    }
    stream += "]";

    return TokenStream::from_str(&stream).unwrap();
}
