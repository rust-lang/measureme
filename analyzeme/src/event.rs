use crate::timestamp::Timestamp;
use std::borrow::Cow;
use std::time::Duration;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Argument<'a> {
    pub name: Option<Cow<'a, str>>,
    pub value: Cow<'a, str>,
}

impl<'a> Argument<'a> {
    pub fn new(value: &'a str) -> Self {
        Self { name: None, value: Cow::from(value) }
    }

    pub fn new_named(name: &'a str, value: &'a str) -> Self {
        Self { name: Some(Cow::from(name)), value: Cow::from(value) }
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Event<'a> {
    pub event_kind: Cow<'a, str>,
    pub label: Cow<'a, str>,
    pub additional_data: Vec<Argument<'a>>,
    pub timestamp: Timestamp,
    pub thread_id: u32,
}

impl<'a> Event<'a> {
    /// Returns true if the time interval of `self` completely contains the
    /// time interval of `other`.
    pub fn contains(&self, other: &Event<'_>) -> bool {
        match self.timestamp {
            Timestamp::Interval {
                start: self_start,
                end: self_end,
            } => match other.timestamp {
                Timestamp::Interval {
                    start: other_start,
                    end: other_end,
                } => self_start <= other_start && other_end <= self_end,
                Timestamp::Instant(other_t) => self_start <= other_t && other_t <= self_end,
            },
            Timestamp::Instant(_) => false,
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        match self.timestamp {
            Timestamp::Interval { start, end } => end.duration_since(start).ok(),
            Timestamp::Instant(_) => None,
        }
    }

    pub(crate) fn parse_event_id(event_id: Cow<'a, str>) -> (Cow<'a, str>, Vec<Argument<'a>>) {
        let event_id = match event_id {
            Cow::Owned(s) => Cow::Owned(s.into_bytes()),
            Cow::Borrowed(s) => Cow::Borrowed(s.as_bytes()),
        };

        let mut parser = Parser::new(event_id);

        let label = match parser.parse_label() {
            Ok(label) => label,
            Err(message) => {
                eprintln!("{}", message);
                return (Cow::from("<parse error>"), Vec::new());
            }
        };

        let mut args = Vec::new();

        while parser.pos != parser.full_text.len() {
            match parser.parse_arg() {
                Ok(arg) => args.push(arg),
                Err(message) => {
                    eprintln!("{}", message);
                    break;
                }
            }
        }

        (label, args)
    }
}

struct Parser<'a> {
    full_text: Cow<'a, [u8]>,
    pos: usize,
}

const ARGUMENT_VALUE_TAG_BYTE: u8 = measureme::event_id::ARGUMENT_VALUE_TAG_BYTE.as_bytes()[0];
const ARGUMENT_NAME_TAG_BYTE: u8 = measureme::event_id::ARGUMENT_NAME_TAG_BYTE.as_bytes()[0];

impl<'a> Parser<'a> {
    fn new(full_text: Cow<'a, [u8]>) -> Parser<'a> {
        Parser { full_text, pos: 0 }
    }

    fn parse_label(&mut self) -> Result<Cow<'a, str>, String> {
        assert!(self.pos == 0);
        let text = self.parse_text()?;
        if text.is_empty() {
            return self.err("<label> is empty");
        } else {
            Ok(text)
        }
    }

    fn parse_text(&mut self) -> Result<Cow<'a, str>, String> {
        let start = self.pos;
        self.pos += self.full_text[start..]
            .iter()
            .take_while(|c| !u8::is_ascii_control(c))
            .count();
        Ok(self.substring(start, self.pos))
    }

    fn parse_arg(&mut self) -> Result<Argument<'a>, String> {
        let name = if let Some(&byte) = self.full_text.get(self.pos) {
            if byte == ARGUMENT_NAME_TAG_BYTE {
                self.pos += 1;
                Some(self.parse_text()?)
            } else {
                None
            }
        } else {
            None
        };
        let value = if let Some(&byte) = self.full_text.get(self.pos) {
            if byte == ARGUMENT_VALUE_TAG_BYTE {
                self.pos += 1;
                Some(self.parse_text()?)
            } else {
                None
            }
        } else {
            None
        };
        match (name, value) {
            (name, Some(value)) => Ok(Argument { name, value }),
            (_, None) => self.err("Unable to parse required <argument_value>"),
        }
    }

    fn err<T>(&self, message: &str) -> Result<T, String> {
        Err(format!(
            r#"Could not parse `event_id`. {} at {} in "{}""#,
            message,
            self.pos,
            std::str::from_utf8(&self.full_text[..]).unwrap()
        ))
    }

    fn substring(&self, start: usize, end: usize) -> Cow<'a, str> {
        match self.full_text {
            Cow::Owned(ref s) => {
                let bytes = s[start..end].to_owned();
                Cow::Owned(String::from_utf8(bytes).unwrap())
            }
            Cow::Borrowed(s) => Cow::Borrowed(std::str::from_utf8(&s[start..end]).unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn parse_event_id_no_args() {
        let (label, args) = Event::parse_event_id(Cow::from("foo"));

        assert_eq!(label, "foo");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_event_id_one_arg() {
        let (label, args) = Event::parse_event_id(Cow::from("foo\x1emy_arg"));

        assert_eq!(label, "foo");
        assert_eq!(args, vec![Argument::new("my_arg")]);
    }

    #[test]
    fn parse_event_id_n_args() {
        let (label, args) = Event::parse_event_id(Cow::from("foo\x1earg1\x1earg2\x1earg3"));

        assert_eq!(label, "foo");
        assert_eq!(
            args,
            vec![Argument::new("arg1"), Argument::new("arg2"), Argument::new("arg3")]
        );
    }

    #[test]
    fn parse_event_id_n_named_args() {
        let (label, args) = Event::parse_event_id(Cow::from("foo\x1darg1\x1eval1\x1darg2\x1eval2"));

        assert_eq!(label, "foo");
        assert_eq!(
            args,
            vec![
                Argument::new_named("arg1", "val1"),
                Argument::new_named("arg2", "val2"),
            ]
        );
    }
}
