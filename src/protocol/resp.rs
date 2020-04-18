use std::default::Default;
use std::option::Option;
use std::result;
use std::str::FromStr;

// TODO: Each `parse_` function should return a value here.
// but I don't handle lifetimes that well yet
#[derive(PartialEq, Debug)]
pub struct Error;

pub type Result<T> = result::Result<T, Error>;

#[derive(PartialEq, Debug)]
pub enum Value {
    NullArray,
    String(String),
    Number(u64),
    Simple(String),
    Error(String),
    Array(Vec<Value>),
}

// simple parsers
fn parse_char(ch: char, input: &str) -> Result<&str> {
    if let Some(x) = input.chars().next() {
        if ch == x {
            return Ok(&input[1..]);
        } else {
            return Err(Error {});
        }
    }
    Err(Error {})
}

fn make_number<T>(input: &str) -> Result<(T, &str)>
where
    T: FromStr,
{
    let digits: String = input.chars().take_while(|x| x.is_digit(10)).collect();
    let result = digits.parse::<T>();
    match result {
        Ok(x) => Ok((x, &input[digits.len()..])),
        Err(_) => Err(Error {}),
    }
}

fn parse_length(s: usize, input: &str) -> Result<(String, &str)> {
    if let Some(x) = input.get(0..s) {
        Ok((x.to_string(), &input[x.len()..]))
    } else {
        Err(Error {})
    }
}

fn ends_with<T>(s: &str, input: (T, &str)) -> Result<T> {
    let x = input.1.replacen(s, "", 1);
    if x.is_empty() {
        Ok(input.0)
    } else {
        Err(Error {})
    }
}

fn has_a<'a, T>(s: &str, input: (T, &'a str)) -> Result<(T, &'a str)> {
    // This is basically a `map'
    if input.1.starts_with(s) {
        Ok((input.0, &(input.1)[s.len()..]))
    } else {
        Err(Error {})
    }
}

impl Value {
    fn parse_int(input: &str) -> Result<Value> {
        parse_char(':', input)
            .and_then(|x| make_number(x))
            .and_then(|x| ends_with("\r\n", x))
            .and_then(|x| Ok(Value::Number(x)))
    }

    fn parse_bulk(input: &str) -> Result<Value> {
        parse_char('$', input)
            .and_then(|x| make_number::<usize>(x))
            .and_then(|x| has_a("\r\n", x))
            .and_then(|x| parse_length(x.0, x.1))
            .and_then(|x| ends_with("\r\n", x))
            .and_then(|x| Ok(Value::String(x)))
    }

    fn parse_array(input: &str) -> Result<Value> {
        parse_char('*', input)
            .and_then(|x| make_number::<u16>(x))
            .and_then(|x| Ok(Value::NullArray))
    }

    pub fn parse(input: &str) -> Result<Value> {
        Value::parse_int(input)
            .or_else(|_| Value::parse_bulk(input))
            .or_else(|_| Value::parse_array(input))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_make_number() {
        assert_eq!(make_number("1aa\r\n"), Ok((1, "aa\r\n")))
    }

    #[test]
    fn test_has_a() {
        assert_eq!(has_a("a", (2, "aa")), Ok((2, "a")));
        assert_eq!(has_a("12", (2, "123456")), Ok((2, "3456")));
    }
    #[test]
    fn test_parse_char() {
        assert_eq!(parse_char('c', "ccaballo"), Ok("caballo"));
        assert_eq!(parse_char('c', "c"), Ok(""));
        assert_eq!(parse_char('a', "caballo"), Err(Error {}));
    }

    #[test]
    fn test_parse_bulk() {
        assert_eq!(
            Value::parse_bulk("$0\r\n\r\n"),
            Ok(Value::String("".to_owned()))
        );
        assert_eq!(
            Value::parse_bulk("$4\r\nRESP\r\n"),
            Ok(Value::String("RESP".to_owned()))
        );
        assert_eq!(Value::parse_bulk("$1\r\n\r\n"), Err(Error {}));
        // We won't handle null strings
        assert_eq!(Value::parse_bulk("$-1\r\n"), Err(Error {}))
    }

    #[test]
    fn test_parse_lenght() {
        let v = "1234567";
        assert_eq!(parse_length(3, v), Ok(("123".to_owned(), "4567")));
        assert_eq!(v.len(), 7);
    }

    #[test]
    fn tes_parse_array() {
        let v = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let data = vec![
            Value::String("foo".to_owned()),
            Value::String("foo".to_owned()),
        ];
        assert_eq!(Value::parse_array(v), Ok(Value::Array(data)))
    }
    #[test]
    fn test_ends_with() {
        assert_eq!(ends_with("aa", (2, "aa")), Ok(2));
        assert_eq!(
            make_number("112\r\n").and_then(|x| ends_with("\r\n", x)),
            Ok(112)
        );
        assert_eq!(
            make_number::<u8>("1aa\r\n").and_then(|x| ends_with("\r\n", x)),
            Err(Error {})
        )
    }
}
