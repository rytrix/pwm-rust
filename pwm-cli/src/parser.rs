pub struct Parser {
    elements: Vec<String>,
}

impl Parser {
    pub fn new(string: &str) -> Parser {
        let mut parser = Parser {
            elements: Vec::new(),
        };

        parser.parse(string);

        parser
    }

    fn parse(&mut self, string: &str) {
        let mut begin = 0;
        let mut end = 0;

        let mut string = string.to_string();

        let get_c = |string: &String, index: usize| {
            let bytes = string.as_bytes();
            bytes[index] as char
        };

        while end < string.len() {
            let mut c = get_c(&string, end);
            if Parser::is_whitespace(c) {
                if begin != end {
                    let data = &string[begin..end];
                    self.elements.push(data.to_string());
                }
                end += 1;
                begin = end;
                continue;
            }
            if c == '\\' {
                if end + 1 < string.len() {
                    c = get_c(&string, end + 1);
                    if c == '\"' {
                        // modify string in place
                        string.remove(end);
                        end += 1;
                        continue;
                    }
                }
            }
            if c == '"' {
                end += 1;
                begin = end;
                c = get_c(&string, end);
                while c != '"' && end < string.len() {
                    end += 1;
                    c = get_c(&string, end);
                }
                if begin != end {
                    let data = &string[begin..end];
                    self.elements.push(data.to_string());
                }
                end += 1;
                begin = end;
                continue;
            }

            end += 1;
        }
        if begin != end {
            let data = &string[begin..end];
            self.elements.push(data.to_string());
        }
    }

    fn is_whitespace(c: char) -> bool {
        c == ' ' || c == '\t' || c == '\r' || c == '\n'
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.elements.iter().map(|s| {
            s.as_str()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;

    #[test]
    fn test_basic() {
        let parser = Parser::new(" insert test 123");
        let mut itr = parser.iter();
        let data = itr.next().unwrap();
        println!("{:?}", data);
        assert_eq!(data, "insert");
        let data = itr.next().unwrap();
        println!("{:?}", data);
        assert_eq!(data, "test");
        let data = itr.next().unwrap();
        println!("{:?}", data);
        assert_eq!(data, "123");
    }

    #[test]
    fn test_extended_name() {
        let parser = Parser::new("insert \"test\" 123");
        let mut itr = parser.iter();

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "insert");
        assert_eq!(data, "insert");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "test");
        assert_eq!(data, "test");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "123");
        assert_eq!(data, "123");
    }

    #[test]
    fn test_extended_name_2() {
        let parser = Parser::new("insert \"test 123\" 123 \"\"");
        let mut itr = parser.iter();

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "insert");
        assert_eq!(data, "insert");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "test 123");
        assert_eq!(data, "test 123");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "123");
        assert_eq!(data, "123");

        let data = itr.next();
        assert_eq!(data, None);
    }

    #[test]
    fn test_escape() {
        let parser = Parser::new("insert \\\"test \\"); 
        let mut itr = parser.iter();

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "insert");
        assert_eq!(data, "insert");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "\"test");
        assert_eq!(data, "\"test");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "\\");
        assert_eq!(data, "\\");
    }

    #[test]
    fn test_random_spacing() {
        let parser = Parser::new("  insert  test"); 
        let mut itr = parser.iter();

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "insert");
        assert_eq!(data, "insert");

        let data = itr.next().unwrap();
        println!("{:?}\n{:?}", data, "test");
        assert_eq!(data, "test");
    }
}
