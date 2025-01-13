use super::kind::{Kind, t};
use crate::source::Span;

const EOF_CHAR: char = '\0';

#[derive(Clone, Copy)]
pub(super) struct Token {
    pub(super) kind: Kind,
    pub(super) span: Span,
}

impl Token {
    pub(super) fn new(kind: Kind, span: Span) -> Self {
        Self { kind, span }
    }
}

pub(super) struct Lexer<'text> {
    text: &'text str,
    chars: core::str::Chars<'text>,
    start: u32,
    current: u32,
}

impl<'text> Lexer<'text> {
    pub(super) fn new(text: &'text str) -> Self {
        Self { text, chars: text.chars(), start: 0, current: 0 }
    }

    fn peek(&self) -> char {
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    fn finish_span(&mut self) -> Span {
        let span = Span::new(self.start, self.current);
        self.start = self.current;
        span
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.current += u32::try_from(c.len_utf8()).unwrap();
        Some(c)
    }

    fn bump_with(&mut self, kind: Kind) -> Kind {
        self.bump();
        kind
    }

    fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while predicate(self.peek()) && !self.is_eof() {
            self.bump();
        }
    }

    fn whitespace(&mut self) -> Kind {
        self.eat_while(char::is_whitespace);
        t![whitespace]
    }

    fn comment(&mut self) -> Kind {
        self.eat_while(|c| c != '\n');
        t![comment]
    }

    fn int(&mut self) -> Kind {
        self.eat_while(|c| c.is_ascii_digit());
        t![int]
    }

    fn word(&mut self, first: char) -> Kind {
        let start = usize::try_from(self.start).unwrap() + 1 - first.len_utf8();
        self.eat_while(unicode_xid::UnicodeXID::is_xid_continue);
        let end = usize::try_from(self.current).unwrap();
        let word = &self.text[start..end];
        match word {
            "true" | "false" => t![bool],
            "fn" => t![fn],
            _ => t![ident],
        }
    }
}

impl Iterator for Lexer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        let first = self.bump()?;
        let kind = match first {
            c if c.is_whitespace() => self.whitespace(),
            '#' => self.comment(),

            '(' => t!['('],
            ')' => t![')'],
            '[' => t!['['],
            ']' => t![']'],
            '{' => t!['{'],
            '}' => t!['}'],

            '.' => t![.],
            ',' => t![,],
            ':' => t![:],
            ';' => t![;],
            '!' => t![!],
            '=' => t![=],
            '-' if self.peek() == '>' => self.bump_with(t![->]),

            '-' if self.peek().is_ascii_digit() => self.int(),
            c if c.is_ascii_digit() => self.int(),
            c if unicode_xid::UnicodeXID::is_xid_start(c) || c == '_' => self.word(c),

            '+' => t![+],
            '-' => t![-],
            '*' => t![*],
            '/' => t![/],

            _ => t![unknown],
        };
        Some(Token::new(kind, self.finish_span()))
    }
}
