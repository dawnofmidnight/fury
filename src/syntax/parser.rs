use super::{
    kind::Kind,
    tree::{Builder, Tree},
};

const FUEL_CAPACITY: u32 = 256;

pub(super) struct Parser<'text> {
    file: crate::source::File,
    text: &'text str,
    lexer: core::iter::Peekable<super::lexer::Lexer<'text>>,
    fuel: core::cell::Cell<u32>,
    events: Vec<Event>,
    diagnostics: Vec<crate::diagnostic::Diagnostic>,
}

impl<'text> Parser<'text> {
    pub(super) fn new(file: crate::source::File, text: &'text str) -> Self {
        Self {
            file,
            text,
            lexer: super::lexer::Lexer::new(text).peekable(),
            fuel: std::cell::Cell::new(FUEL_CAPACITY),
            events: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub(super) fn file(&self) -> crate::source::File {
        self.file
    }

    pub(super) fn at_end(&mut self) -> bool {
        self.lexer.peek().is_none()
    }

    fn peek_token(&mut self) -> Option<super::lexer::Token> {
        self.lexer.peek().copied()
    }

    pub(super) fn peek_span(&mut self) -> crate::source::Span {
        self.peek_token().map_or_else(
            || {
                let len = u32::try_from(self.text.len()).unwrap();
                crate::source::Span::new(len, len)
            },
            |t| t.span,
        )
    }

    pub(super) fn peek(&mut self) -> Kind {
        assert!(self.fuel.get() != 0, "parser is stuck");
        self.fuel.set(self.fuel.get() - 1);
        self.peek_token().map_or(Kind::Eof, |t| t.kind)
    }

    fn bump_raw(&mut self) {
        self.fuel.set(FUEL_CAPACITY);
        let Some(super::lexer::Token { kind, span }) = self.lexer.next() else {
            panic!("tried to consume nonexistent token");
        };
        self.events.push(Event::Token { kind, length: span.length() });
    }

    pub(super) fn bump(&mut self) {
        self.bump_raw();
        self.eat_trivia();
    }

    pub(super) fn eat_trivia(&mut self) {
        while self.peek().is_trivia() {
            self.bump_raw();
        }
    }

    pub(super) fn open(&mut self) -> MarkOpened {
        let mark = MarkOpened { index: self.events.len() };
        self.events.push(Event::Open { kind: Kind::Error });
        mark
    }

    pub(super) fn close(&mut self, marker: MarkOpened, kind: Kind) {
        let num_trivia_before = self
            .events
            .iter()
            .rev()
            .take_while(|e| matches!(e, Event::Token { kind, ..} if kind.is_trivia()))
            .count();
        self.events[marker.index] = Event::Open { kind };
        self.events.insert(self.events.len() - num_trivia_before, Event::Close);
    }

    pub(super) fn expect(&mut self, expected: Kind) {
        let found = self.peek();
        if found == expected {
            self.bump();
        } else {
            let span = self.peek_span();
            self.diagnostic(errors::unexpected_token(self.file, span, &[expected], None, found));
        }
    }

    pub(super) fn optional(&mut self, kind: Kind) -> bool {
        if self.peek() == kind {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(super) fn diagnostic(&mut self, diagnostic: crate::diagnostic::Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn finish(self) -> (Tree, Vec<crate::diagnostic::Diagnostic>) {
        let Self { events, diagnostics, .. } = self;
        let mut builder = Builder::new();
        for event in events {
            match event {
                Event::Open { kind } => builder.open(kind),
                Event::Close => builder.close(),
                Event::Token { kind, length } => builder.token(kind, length),
            }
        }
        (builder.build(), diagnostics)
    }
}

enum Event {
    Open { kind: Kind },
    Close,
    Token { kind: Kind, length: u32 },
}

#[derive(Clone, Copy)]

pub(super) struct MarkOpened {
    index: usize,
}

macro_rules! expected {
    ($parser:expr, $found:expr $(, $matcher:tt)+ $(,)? $(($phrase:literal))?) => {{
        let span = $parser.peek_span();
        let expected = &[$(crate::syntax::kind::t![$matcher]),+];
        $parser.diagnostic(
            $crate::syntax::parser::errors::unexpected_token(
                $parser.file(),
                span,
                expected,
                crate::syntax::parser::expected!(@phrase $($phrase)?),
                $found,
            )
        );
    }};
    (@phrase $phrase:literal) => { Some($phrase) };
    (@phrase) => { None };
}
pub(super) use expected;

pub(super) mod errors {
    use core::fmt::Write as _;

    pub(in super::super) fn unexpected_token(
        file: crate::source::File,
        span: crate::source::Span,
        expected: &[crate::syntax::kind::Kind],
        expected_phrase: Option<&'static str>,
        found: crate::syntax::kind::Kind,
    ) -> crate::diagnostic::Diagnostic {
        let expected_format = match expected_phrase {
            Some(phrase) => std::borrow::Cow::Borrowed(phrase),
            None => std::borrow::Cow::Owned(list_format(expected)),
        };
        let mut diagnostic = crate::diagnostic::Diagnostic::error(format!(
            "expected {expected_format}, found {found}"
        ));
        diagnostic.primary(file, span, format!("found {found} here"));
        if let Some(phrase) = expected_phrase {
            diagnostic.note(format!("{phrase} can start with {}", list_format(expected)));
        }
        diagnostic
    }

    fn list_format<T: core::fmt::Display>(list: &[T]) -> String {
        match list.len() {
            0 => panic!("attempted to create a list of 0 items"),
            1 => list[0].to_string(),
            2 => format!("{} or {}", list[0], list[1]),
            3.. => {
                let mut output = String::new();
                for ty in &list[0..list.len() - 2] {
                    write!(output, "{ty}, ").unwrap();
                }
                write!(output, "{}, or {}", list[list.len() - 2], list[list.len() - 1]).unwrap();
                output
            }
        }
    }
}
