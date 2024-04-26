extern crate proc_macro;
extern crate getrandom;
use std::str::FromStr;

use getrandom::getrandom;
use proc_macro::TokenStream;

#[proc_macro]
pub fn random_number(_input: TokenStream) -> TokenStream {
    let mut random: [u8; 32] = [0; 32];
    getrandom(&mut random).unwrap();

    let mut stream = "[".to_string();
    for num in random {
        stream += format!("{},", num).as_str();
    }
    stream += "]";

    TokenStream::from_str(&stream).unwrap()
}
