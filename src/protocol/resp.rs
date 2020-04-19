use std::default::Default;
use std::result;
use std::str::FromStr;

// TODO: Each `parse_` function should return a value here.
// but I don't handle lifetimes that well yet
#[derive(PartialEq, Debug)]
pub struct Error {
    what: String,
}

pub type Result<T> = result::Result<T, Error>;

#[derive(PartialEq, Debug)]
pub enum Value {
    NullArray,
    String(String),
    Number(u64),
    Array(Vec<Value>),
}

fn repeat<T, P>(n: u16, mut input: &str, mut f: P) -> Result<(Vec<T>, &str)>
where
    P: FnMut(&str) -> Result<(T, &str)>,
    T: Default,
{
    let mut result = Vec::new();
    for _ in 0..n {
        match f(input) {
            Ok((next_item, next_input)) => {
                input = next_input;
                result.push(next_item)
            }
            Err(x) => return Err(x),
        }
    }
    Ok((result, input))
}

// simple parsers
fn parse_char(ch: char, input: &str) -> Result<&str> {
    if let Some(x) = input.chars().next() {
        if ch == x {
            return Ok(&input[1..]);
        } else {
            return Err(Error {
                what: input.to_owned(),
            });
        }
    }
    Err(Error {
        what: input.to_owned(),
    })
}

fn make_number<T>(input: &str) -> Result<(T, &str)>
where
    T: FromStr,
{
    let digits: String = input.chars().take_while(|x| x.is_digit(10)).collect();
    let result = digits.parse::<T>();
    match result {
        Ok(x) => Ok((x, &input[digits.len()..])),
        Err(_) => Err(Error {
            what: input.to_owned(),
        }),
    }
}

fn parse_length(s: usize, input: &str) -> Result<(String, &str)> {
    if let Some(x) = input.get(0..s) {
        Ok((x.to_string(), &input[x.len()..]))
    } else {
        Err(Error {
            what: input.to_owned(),
        })
    }
}

fn match_exact<'a, T>(s: &str, input: (T, &'a str)) -> Result<(T, &'a str)> {
    match input.1.get(0..s.len()) {
        Some(x) if x == s => Ok((input.0, &input.1[s.len()..])),
        Some(_) => Err(Error {
            what: input.1.to_owned(),
        }),
        None => Err(Error {
            what: input.1.to_owned(),
        }),
    }
}

fn has_a<'a, T>(s: &str, input: (T, &'a str)) -> Result<(T, &'a str)> {
    if input.1.starts_with(s) {
        Ok((input.0, &(input.1)[s.len()..]))
    } else {
        Err(Error {
            what: input.1.to_owned(),
        })
    }
}

impl Default for Value {
    fn default() -> Value {
        Value::NullArray
    }
}
impl Value {
    fn parse_int(input: &str) -> Result<(Value, &str)> {
        parse_char(':', input)
            .and_then(|x| make_number(x))
            .and_then(|x| match_exact("\r\n", x))
            .and_then(|x| Ok((Value::Number(x.0), x.1)))
    }

    fn parse_bulk(input: &str) -> Result<(Value, &str)> {
        parse_char('$', input)
            .and_then(|x| make_number::<usize>(x))
            .and_then(|x| has_a("\r\n", x))
            .and_then(|x| parse_length(x.0, x.1))
            .and_then(|x| match_exact("\r\n", x))
            .and_then(|x| Ok((Value::String(x.0), x.1)))
    }

    fn parse_array(input: &str) -> Result<(Value, &str)> {
        parse_char('*', input)
            .and_then(|x| make_number::<u16>(x))
            .and_then(|x| match_exact("\r\n", x))
            .and_then(|x| repeat(x.0, x.1, Value::parse))
            .and_then(|x| Ok((Value::Array(x.0), x.1)))
    }

    pub fn parse(input: &str) -> Result<(Value, &str)> {
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
        assert_eq!(
            parse_char('a', "caballo"),
            Err(Error {
                what: "caballo".to_owned()
            })
        );
    }

    #[test]
    fn test_parse_bulk() {
        assert_eq!(
            Value::parse_bulk("$0\r\n\r\n"),
            Ok((Value::String("".to_owned()), ""))
        );
        assert_eq!(
            Value::parse_bulk("$4\r\nRESP\r\n"),
            Ok((Value::String("RESP".to_owned()), ""))
        );
        assert_eq!(
            Value::parse_bulk("$1\r\n\r\n"),
            Err(Error {
                what: "\n".to_owned()
            })
        );
        // We won't handle null strings
        assert_eq!(
            Value::parse_bulk("$-1\r\n"),
            Err(Error {
                what: "-1\r\n".to_owned()
            })
        )
    }

    #[test]
    fn test_parse_lenght() {
        let v = "1234567";
        assert_eq!(parse_length(3, v), Ok(("123".to_owned(), "4567")));
        assert_eq!(v.len(), 7);
    }

    #[test]
    fn tes_parse_array() {
        // simple array
        let v = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let data = vec![
            Value::String("foo".to_owned()),
            Value::String("bar".to_owned()),
        ];
        assert_eq!(Value::parse_array(v), Ok((Value::Array(data), "")));
        //empty array
        let v1 = "*0\r\n";
        assert_eq!(Value::parse_array(v1), Ok((Value::Array(Vec::new()), "")));
        // null array
        let v2 = "*-1\r\n";
        assert_eq!(
            Value::parse_array(v2),
            Err(Error {
                what: "-1\r\n".to_owned()
            })
        );
        // nested array
        let v3 = "*2\r\n*1\r\n:5\r\n*3\r\n:4\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let data3 = vec![
            Value::Array(vec![Value::Number(5)]),
            Value::Array(vec![
                Value::Number(4),
                Value::String("foo".to_owned()),
                Value::String("bar".to_owned()),
            ]),
        ];
        assert_eq!(Value::parse(v3), Ok((Value::Array(data3), "")))
    }

    #[test]
    fn test_repeat() {
        let v = ":2\r\n:3\r\n:4\r\n";
        let p = repeat(3, v, |s| {
            parse_char(':', s)
                .and_then(|x| make_number(x))
                .and_then(|x| match_exact("\r\n", x))
        });
        assert_eq!(p, Ok((vec![2, 3, 4], "")));
        assert_eq!(v, ":2\r\n:3\r\n:4\r\n");
    }
    #[test]
    fn test_match_exact() {
        assert_eq!(match_exact("aa", (2, "aa")), Ok((2, "")));
        assert_eq!(
            make_number("112\r\n").and_then(|x| match_exact("\r\n", x)),
            Ok((112, ""))
        );
        assert_eq!(
            make_number::<u8>("1aa\r\n").and_then(|x| match_exact("\r\n", x)),
            Err(Error {
                what: "aa\r\n".to_owned()
            })
        )
    }
}
