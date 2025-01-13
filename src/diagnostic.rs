use crate::source::{File, Span};
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Diagnostic {
    pub(crate) severity: Severity,
    pub(crate) message: Cow<'static, str>,
    pub(crate) primary: Option<Label>,
    pub(crate) secondary: Vec<Label>,
    pub(crate) notes: Vec<Cow<'static, str>>,
}

impl Diagnostic {
    #[must_use]
    pub(crate) fn error(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            primary: None,
            secondary: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub(crate) fn primary(
        &mut self,
        file: File,
        span: Span,
        message: impl Into<Cow<'static, str>>,
    ) -> &mut Self {
        debug_assert!(self.primary.is_none(), "primary diagnostic already set");
        self.primary = Some(Label { file, span, message: message.into() });
        self
    }

    // pub(crate) fn secondary(
    //     &mut self,
    //     file: File,
    //     span: Span,
    //     message: impl Into<Cow<'static, str>>,
    // ) -> &mut Self {
    //     self.secondary.push(Label { file, span, message: message.into() });
    //     self
    // }

    pub(crate) fn note(&mut self, message: impl Into<Cow<'static, str>>) -> &mut Self {
        self.notes.push(message.into());
        self
    }
}

impl<E: core::error::Error> From<E> for Diagnostic {
    fn from(error: E) -> Self {
        Self {
            severity: Severity::Error,
            message: Cow::Owned(error.to_string()),
            primary: None,
            secondary: Vec::new(),
            notes: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(dead_code)]
pub(crate) enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl core::fmt::Display for Severity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Error => f.write_str("error"),
            Self::Warning => f.write_str("warning"),
            Self::Note => f.write_str("note"),
            Self::Help => f.write_str("help"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Label {
    pub(crate) file: File,
    pub(crate) span: Span,
    pub(crate) message: Cow<'static, str>,
}
