use super::timestamp::Timestamp;
use memchr::memchr;
use std::borrow::Cow;
use std::time::Duration;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Event<'a> {
    pub event_kind: Cow<'a, str>,
    pub label: Cow<'a, str>,
    pub additional_data: Vec<Cow<'a, str>>,
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

    pub(crate) fn parse_event_id(event_id: Cow<'a, str>) -> (Cow<'a, str>, Vec<Cow<'a, str>>) {
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

const SEPARATOR_BYTE: u8 = super::measureme_9_2_0::event_id::SEPARATOR_BYTE.as_bytes()[0];

impl<'a> Parser<'a> {
    fn new(full_text: Cow<'a, [u8]>) -> Parser<'a> {
        Parser { full_text, pos: 0 }
    }

    fn peek(&self) -> u8 {
        self.full_text[self.pos]
    }

    fn parse_label(&mut self) -> Result<Cow<'a, str>, String> {
        assert!(self.pos == 0);
        self.parse_separator_terminated_text()
    }

    fn parse_separator_terminated_text(&mut self) -> Result<Cow<'a, str>, String> {
        let start = self.pos;

        let end = memchr(SEPARATOR_BYTE, &self.full_text[start..])
            .map(|pos| pos + start)
            .unwrap_or(self.full_text.len());

        if start == end {
            return self.err("Zero-length <text>");
        }

        self.pos = end;

        if self.full_text[start..end].iter().any(u8::is_ascii_control) {
            return self.err("Found ASCII control character in <text>");
        }

        Ok(self.substring(start, end))
    }

    fn parse_arg(&mut self) -> Result<Cow<'a, str>, String> {
        if self.peek() != SEPARATOR_BYTE {
            return self.err(&format!(
                "Expected '\\x{:x}' char at start of <argument>",
                SEPARATOR_BYTE
            ));
        }

        self.pos += 1;
        self.parse_separator_terminated_text()
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
        assert_eq!(args, vec![Cow::from("my_arg")]);
    }

    #[test]
    fn parse_event_id_n_args() {
        let (label, args) = Event::parse_event_id(Cow::from("foo\x1earg1\x1earg2\x1earg3"));

        assert_eq!(label, "foo");
        assert_eq!(
            args,
            vec![Cow::from("arg1"), Cow::from("arg2"), Cow::from("arg3")]
        );
    }
}
