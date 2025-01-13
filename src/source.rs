// We use `line_index` a lot internally here, but it should not be publicly
// exposed; we might replace it at a later time.

pub(crate) use file::File;
pub(crate) use span::Span;

#[salsa::input]
pub struct Source {
    #[return_ref]
    pub files: Vec<File>,
}

paracord::custom_key!(
    pub(crate) struct Symbol;
);

impl Default for Symbol {
    fn default() -> Self {
        Symbol::new("")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LocationUtf8 {
    pub(crate) line: u32,
    pub(crate) column: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LocationUtf16 {
    pub(crate) line: u32,
    pub(crate) column: u32,
}

impl core::fmt::Display for LocationUtf8 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

mod file {
    // TODO: we assume `text.len() < u32::MAX`. enforce that invariant upon
    // construction with salsa.
    #[salsa::input]
    #[derive(Debug)]
    pub struct File {
        #[return_ref]
        pub(crate) path: std::path::PathBuf,
        #[return_ref]
        pub(crate) text: String,
    }

    #[salsa::tracked]
    impl File {
        fn line_index(self, db: &dyn crate::Db) -> line_index::LineIndex {
            line_index::LineIndex::new(self.text(db))
        }

        pub(crate) fn line(self, db: &dyn crate::Db, line: u32) -> &str {
            &self.text(db)[self.line_index(db).line(line - 1).unwrap()]
        }

        pub(crate) fn index_to_location_utf16(
            self,
            db: &dyn crate::Db,
            index: u32,
        ) -> super::LocationUtf16 {
            let line_col = self.line_index(db).line_col(index.into());
            super::LocationUtf16 { line: line_col.line + 1, column: line_col.col + 1 }
        }

        pub(crate) fn location_utf16_to_index(
            self,
            db: &dyn crate::Db,
            location: super::LocationUtf16,
        ) -> u32 {
            let index = self.line_index(db);
            let wide_line_col =
                line_index::WideLineCol { line: location.line - 1, col: location.column - 1 };
            let line_col = index.to_utf8(line_index::WideEncoding::Utf16, wide_line_col).unwrap();
            index.offset(line_col).unwrap().into()
        }

        pub(crate) fn location_utf8(self, db: &dyn crate::Db, index: u32) -> super::LocationUtf8 {
            let line_col = self.line_index(db).line_col(line_index::TextSize::new(index));
            super::LocationUtf8 { line: line_col.line + 1, column: line_col.col + 1 }
        }
    }
}

mod span {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct Span {
        pub(crate) start: u32,
        pub(crate) end: u32,
    }
    const _: () = assert!(size_of::<Span>() == 8);

    impl Span {
        #[must_use]
        pub(crate) const fn new(start: u32, end: u32) -> Self {
            debug_assert!(start <= end);
            Self { start, end }
        }

        pub(crate) fn length(self) -> u32 {
            self.end - self.start
        }

        pub(crate) fn join(self, other: Span) -> Span {
            Self { start: self.start.min(other.start), end: self.end.max(other.end) }
        }
    }

    impl core::fmt::Display for Span {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{}..{}", self.start, self.end)
        }
    }

    impl core::ops::Index<Span> for str {
        type Output = str;

        fn index(&self, span: Span) -> &str {
            &self[usize::try_from(span.start).unwrap()..usize::try_from(span.end).unwrap()]
        }
    }
}
