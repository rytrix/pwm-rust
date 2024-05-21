use std::ops::Range;

struct Parser {
    elements: Vec<Range<*const u8>>,
    data: Vec<u8>,
}

impl Parser {
    pub fn new(string: &str) -> Parser {
        let mut parser = Parser {
            elements: Vec::new(),
            data: Vec::new(),
        };

        parser.parse(string);

        parser
    }

    fn parse(&mut self, string: &str) {
        let mut begin = 0;
        let mut end = 0;

        let byte_string = string.as_bytes();
        let mut byte_string_iter = 0;

        while byte_string_iter < byte_string.len() {
            let mut c = byte_string[byte_string_iter] as char;
            if Parser::is_whitespace(c) {
                if begin != end {
                    let data = &self.data[begin..end];
                    println!("debug: {:?}", String::from_utf8(data.to_vec()));
                    self.elements.push(data.as_ptr_range());
                }
                byte_string_iter += 1;
                begin = end;
                continue;
            }
            if c == '\\' {
                c = byte_string[byte_string_iter + 1] as char;
                if c == '\"' {
                    byte_string_iter += 2;
                    self.data.push(b'\"');
                    end += 1;
                    continue;
                }
            }
            if c == '"' {
                byte_string_iter += 1;
                c = byte_string[byte_string_iter] as char;
                while c != '"' && byte_string_iter < byte_string.len() {
                    end += 1;
                    self.data.push(c as u8);
                    byte_string_iter += 1;
                    c = byte_string[byte_string_iter] as char;
                }
                let data = &self.data[begin..end];
                println!("debug: {:?}", String::from_utf8(data.to_vec()));
                self.elements.push(data.as_ptr_range());
                byte_string_iter += 1;
                begin = end;
                continue;
            }

            byte_string_iter += 1;
            end += 1;
            self.data.push(c as u8);
        }
        let data = &self.data[begin..end];
        println!("debug: {:?}", String::from_utf8(data.to_vec()));
        self.elements.push(data.as_ptr_range());
    }

    fn is_whitespace(c: char) -> bool {
        c == ' ' || c == '\t' || c == '\r' || c == '\n'
    }

    pub fn iter(&self) -> impl Iterator<Item = &[u8]> {
        let itr = self.elements.iter().map(|r| unsafe {
            let len = r.end as usize - r.start as usize;
            println!("range length {}", len);
            let slice = std::slice::from_raw_parts(r.start, len);
            println!("slice {:?}", slice);
            slice
        });

        itr
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;

    #[test]
    fn test_basic() {
        let parser = Parser::new("insert test 123");
        let mut itr = parser.iter();
        let data = itr.next().unwrap();
        println!("{:?}", data);
        assert_eq!(data, b"insert");
        let data = itr.next().unwrap();
        println!("{:?}", data);
        assert_eq!(data, b"test");
        let data = itr.next().unwrap();
        println!("{:?}", data);
        assert_eq!(data, b"123");
    }

    #[test]
    fn test_extended_name() {
        let parser = Parser::new("insert \"test\" 123");
        let mut itr = parser.iter();

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, b"insert");
        assert_eq!(data, b"insert");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, b"test");
        assert_eq!(data, b"test");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, b"123");
        assert_eq!(data, b"123");
    }

    #[test]
    fn test_extended_name_2() {
        let parser = Parser::new("insert \"test 123\" 123");
        let mut itr = parser.iter();

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", String::from_utf8(data.to_vec()), "insert");
        assert_eq!(data, b"insert");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", String::from_utf8(data.to_vec()), "test 123");
        assert_eq!(data, b"test 123");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", String::from_utf8(data.to_vec()), "123");
        assert_eq!(data, b"123");
    }
}
