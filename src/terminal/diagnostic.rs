#![expect(dead_code)]

//! Rendering of diagnostics for terminal output.
//!
//! The rendering algorithm is not particularly efficient, but that is fine
//! because this is only expected to be done once per compiler invocation.

pub(crate) fn emit<Writer>(
    db: &dyn crate::Db,
    diagnostics: &[crate::diagnostic::Diagnostic],
    writer: &mut Writer,
) -> std::io::Result<()>
where
    Writer: termcolor::WriteColor,
{
    let mut renderer = renderer::Renderer::new(db, writer);
    for diagnostic in diagnostics {
        let snippets = produce_snippets(db, diagnostic.primary.as_ref(), &diagnostic.secondary);
        let gutter_padding =
            snippets.iter().map(snippet::Snippet::gutter_padding).max().unwrap_or(0);
        renderer.header(diagnostic.severity, &diagnostic.message)?;
        for snippet in snippets {
            emit_snippet(db, &mut renderer, snippet, gutter_padding)?;
        }
        for note in &diagnostic.notes {
            renderer.note(note, gutter_padding)?;
        }
        renderer.footer()?;
    }
    Ok(())
}

fn produce_snippets<'db, 'diagnostic>(
    db: &'db dyn crate::Db,
    primary: Option<&'diagnostic crate::diagnostic::Label>,
    secondary: &'diagnostic [crate::diagnostic::Label],
) -> Vec<snippet::Snippet<'db, 'diagnostic>> {
    let mut snippets = Vec::<snippet::Snippet<'_, '_>>::new();
    for (label, primary) in
        primary.map(|l| (l, true)).into_iter().chain(secondary.iter().map(|l| (l, false)))
    {
        match snippets.iter_mut().find(|s| s.file == label.file) {
            Some(snippet) => snippet.insert(db, label, primary),
            None => {
                let mut snippet = snippet::Snippet::new(label.file);
                snippet.insert(db, label, primary);
                snippets.push(snippet);
            }
        }
    }
    snippets
}

fn emit_snippet<Writer>(
    db: &dyn crate::Db,
    renderer: &mut renderer::Renderer<'_, '_, Writer>,
    snippet: snippet::Snippet<'_, '_>,
    gutter_padding: usize,
) -> std::io::Result<()>
where
    Writer: termcolor::WriteColor,
{
    renderer.snippet_header(snippet.file.path(db), snippet.main_location(), gutter_padding)?;
    for (number, line) in snippet.lines {
        renderer.snippet_line(number, line.content, gutter_padding)?;
    }
    renderer.snippet_footer(gutter_padding)?;
    Ok(())
}

mod snippet {
    /// A label which is entirely on a single line with respect to its
    /// column-based positions.
    pub(super) struct SingleLabel<'diagnostic> {
        pub(super) primary: bool,
        pub(super) start: u32,
        pub(super) end: u32,
        pub(super) message: &'diagnostic str,
    }

    #[derive(Default)]
    pub(super) struct Line<'db, 'diagnostic> {
        pub(super) content: &'db str,
        pub(super) single: Vec<SingleLabel<'diagnostic>>,
    }

    pub(super) struct Snippet<'db, 'diagnostic> {
        pub(super) file: crate::source::File,
        pub(super) lines: std::collections::BTreeMap<u32, Line<'db, 'diagnostic>>,
    }

    impl<'db, 'diagnostic> Snippet<'db, 'diagnostic> {
        pub(super) fn new(file: crate::source::File) -> Self {
            Self { file, lines: std::collections::BTreeMap::new() }
        }

        pub(super) fn main_location(&self) -> crate::source::LocationUtf8 {
            let lines = self.lines.iter();
            let single_kind_locations = lines.flat_map(|(&line, l)| {
                l.single.iter().map(move |sl| {
                    (sl.primary, crate::source::LocationUtf8 { line, column: sl.start })
                })
            });
            let primary_locations = single_kind_locations
                .clone()
                .filter_map(|(primary, location)| primary.then_some(location));
            match primary_locations.min() {
                Some(min) => min,
                None => single_kind_locations.map(|(_, location)| location).min().unwrap(),
            }
        }

        pub(super) fn gutter_padding(&self) -> usize {
            usize::try_from(self.lines.last_key_value().map_or(1, |(&k, _)| k).ilog10()).unwrap()
                + 1
        }

        pub(super) fn insert(
            &mut self,
            db: &'db dyn crate::Db,
            label: &'diagnostic crate::diagnostic::Label,
            primary: bool,
        ) {
            let start = self.file.location_utf8(db, label.span.start);
            let end = self.file.location_utf8(db, label.span.end);
            if start.line == end.line {
                self.lines
                    .entry(start.line)
                    .or_insert(Line {
                        content: label.file.line(db, start.line),
                        single: Vec::new(),
                    })
                    .single
                    .push(SingleLabel {
                        primary,
                        start: start.column,
                        end: end.column,
                        message: &label.message,
                    });
            } else {
                println!("start: {start}, end: {end}");
                todo!()
            }
        }
    }
}

mod renderer {
    use crate::diagnostic::Severity;
    use termcolor::{Color, ColorSpec};

    pub(super) struct Renderer<'db, 'writer, Writer> {
        db: &'db dyn crate::Db,
        writer: &'writer mut Writer,
        styles: Styles,
    }

    impl<'db, 'writer, Writer> Renderer<'db, 'writer, Writer>
    where
        Writer: termcolor::WriteColor,
    {
        pub(super) fn new(db: &'db dyn crate::Db, writer: &'writer mut Writer) -> Self {
            Self { db, writer, styles: Styles::default() }
        }

        pub(super) fn header(&mut self, severity: Severity, message: &str) -> std::io::Result<()> {
            self.writer.set_color(self.styles.header(severity))?;
            write!(self.writer, "{severity}")?;
            self.writer.set_color(&self.styles.header_message)?;
            writeln!(self.writer, ": {message}")?;
            self.writer.reset()?;
            Ok(())
        }

        pub(super) fn footer(&mut self) -> std::io::Result<()> {
            writeln!(self.writer)
        }

        fn gutter_numbered(&mut self, line: u32, gutter_padding: usize) -> std::io::Result<()> {
            self.writer.set_color(&self.styles.frame)?;
            write!(self.writer, "{line: >gutter_padding$} {} ", chars::VERTICAL)?;
            self.writer.reset()
        }

        fn gutter_gap(&mut self, gutter_padding: usize) -> std::io::Result<()> {
            self.writer.set_color(&self.styles.frame)?;
            write!(self.writer, "{: >gutter_padding$} {} ", "", chars::GAP)?;
            self.writer.reset()
        }

        pub(super) fn snippet_header(
            &mut self,
            path: &std::path::Path,
            location: crate::source::LocationUtf8,
            gutter_padding: usize,
        ) -> std::io::Result<()> {
            self.writer.set_color(&self.styles.frame)?;
            write!(
                self.writer,
                "{: >gutter_padding$} {}{}[",
                "",
                chars::TOP_LEFT,
                chars::HORIZONTAL
            )?;
            self.writer.reset()?;
            write!(self.writer, "{}:{location}", path.display())?;
            self.writer.set_color(&self.styles.frame)?;
            write!(self.writer, "]")?;
            self.writer.reset()?;
            writeln!(self.writer)
        }

        pub(super) fn snippet_line(
            &mut self,
            number: u32,
            content: &str,
            gutter_padding: usize,
        ) -> std::io::Result<()> {
            self.gutter_numbered(number, gutter_padding)?;
            writeln!(self.writer, "{content}")
        }

        pub(super) fn snippet_footer(&mut self, gutter_padding: usize) -> std::io::Result<()> {
            self.writer.set_color(&self.styles.frame)?;
            writeln!(self.writer, "{: >gutter_padding$} {}", "", chars::VERTICAL)?;
            self.writer.reset()
        }

        pub(super) fn note(&mut self, note: &str, gutter_padding: usize) -> std::io::Result<()> {
            self.writer.set_color(&self.styles.frame)?;
            write!(self.writer, "{: >gutter_padding$} = ", "")?;
            self.writer.reset()?;
            writeln!(self.writer, "{note}")
        }
    }

    pub(super) struct Styles {
        pub(super) frame: ColorSpec,
        pub(super) header_error: ColorSpec,
        pub(super) header_help: ColorSpec,
        pub(super) header_message: ColorSpec,
        pub(super) header_note: ColorSpec,

        pub(super) header_warning: ColorSpec,
        pub(super) primary_error: ColorSpec,
        pub(super) primary_help: ColorSpec,
        pub(super) primary_note: ColorSpec,
        pub(super) primary_warning: ColorSpec,

        pub(super) secondary: ColorSpec,
    }

    impl Styles {
        pub(super) fn header(&self, severity: Severity) -> &ColorSpec {
            match severity {
                Severity::Error => &self.header_error,
                Severity::Warning => &self.header_warning,
                Severity::Note => &self.header_note,
                Severity::Help => &self.header_help,
            }
        }

        pub(super) fn primary(&self, severity: Severity) -> &ColorSpec {
            match severity {
                Severity::Error => &self.primary_error,
                Severity::Warning => &self.primary_warning,
                Severity::Note => &self.primary_note,
                Severity::Help => &self.primary_help,
            }
        }
    }

    impl Default for Styles {
        fn default() -> Self {
            let header = ColorSpec::new().set_bold(true).set_intense(true).clone();
            Self {
                header_error: header.clone().set_fg(Some(Color::Red)).clone(),
                header_warning: header.clone().set_fg(Some(Color::Yellow)).clone(),
                header_note: header.clone().set_fg(Some(Color::Cyan)).clone(),
                header_help: header.clone().set_fg(Some(Color::Green)).clone(),
                header_message: header,

                primary_error: ColorSpec::new().set_fg(Some(Color::Red)).clone(),
                primary_warning: ColorSpec::new().set_fg(Some(Color::Yellow)).clone(),
                primary_note: ColorSpec::new().set_fg(Some(Color::Cyan)).clone(),
                primary_help: ColorSpec::new().set_fg(Some(Color::Green)).clone(),
                secondary: ColorSpec::new().set_fg(Some(Color::Blue)).clone(),

                frame: ColorSpec::new().set_fg(Some(Color::Blue)).clone(),
            }
        }
    }

    mod chars {
        pub(super) const VERTICAL: char = '│';
        pub(super) const HORIZONTAL: char = '─';

        pub(super) const GAP: char = '·';

        // pub(super) const TOP_JOINER: char = '┬';
        // pub(super) const BOTTOM_JOINER: char = '┴';
        // pub(super) const LEFT_JOINER: char = '├';
        // pub(super) const RIGHT_JOINER: char = '┤';

        pub(super) const TOP_LEFT: char = '╭';
        // pub(super) const TOP_RIGHT: char = '╮';
        // pub(super) const BOTTOM_LEFT: char = '╰';
        // pub(super) const BOTTOM_RIGHT: char = '╯';
    }
}

#[cfg(test)]
pub(crate) struct StringWriter(pub(crate) String);

#[cfg(test)]
impl std::io::Write for StringWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let Ok(s) = str::from_utf8(buf) else {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        };
        self.0.push_str(s);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::{
//         diagnostic::{Diagnostic, Label, LabelKind, Severity},
//         source::{File, Span},
//     };

//     // lorem ipsum text does
//     const SOURCE_A: &str = "\
//         lorem ipsum dolor sit amet consectetur adipiscing\
//         tempor incididunt ut labore et dolore magna\
//         veniam quis nostrud exercitation ullamco laboris\
//         commodo consequat duis aute irure dolor in\
//         velit esse cillum dolore eu fugiat nulla pariatur\
//         occaecat cupidatat non proident sunt in culpa qui\
//         mollit anim id est laborum\
//     ";

//     const SOURCE_B: &str = "\
//         penatibus et magnis dis parturient montes nascetur\
//         efficitur laoreet mauris pharetra vestibulum fusce\
//         facilisis dapibus etiam interdum tortor ligula\
//         dignissim velit aliquam imperdiet mollis praesent\
//         ultrices proin libero feugiat hac habitasse platea\
//         dolor sit mi pretium tellus duis convallis tempus\
//         lacinia integer nunc posuere ut hendrerit\
//     ";

//     fn render(diagnostic: impl FnOnce(File, File) -> Diagnostic) -> String {
//         let db = crate::Database::default();
//         let mut writer = StringWriter(String::new());
//         let a = File::new(&db, "test_a.txt".into(), SOURCE_A.to_owned());
//         let b = File::new(&db, "test_b.txt".into(), SOURCE_B.to_owned());
//         super::emit(&db, &[diagnostic(a, b)], &mut
// termcolor::NoColor::new(&mut writer)).unwrap();         writer.0
//     }

//     fn primary(file: File, start: u32, end: u32, message: &'static str) ->
// Label {         Label {
//             kind: LabelKind::Primary,
//             file,
//             span: Span::new(start, end),
//             message: std::borrow::Cow::Borrowed(message),
//         }
//     }

//     fn secondary(file: File, start: u32, end: u32, message: &'static str) ->
// Label {         Label {
//             kind: LabelKind::Secondary,
//             file,
//             span: Span::new(start, end),
//             message: std::borrow::Cow::Borrowed(message),
//         }
//     }

//     #[test]
//     fn message_only() {
//         insta::assert_snapshot!(
//             render(|_, _| Diagnostic {
//                 severity: Severity::Error,
//                 message: "test message".into(),
//                 labels: Vec::new(),
//                 notes: Vec::new(),
//             }),
//             @"error: test message",
//         );
//     }

//     #[test]
//     fn message_and_notes() {
//         insta::assert_snapshot!(
//             render(|_, _| Diagnostic {
//                 severity: Severity::Error,
//                 message: "test message".into(),
//                 labels: Vec::new(),
//                 notes: vec!["note 1".into(), "note 2".into()],
//             }),
//             @"
//         error: test message
//          = note 1
//          = note 2
//         ",
//         );
//     }

//     #[test]
//     fn single_line_label() {
//         insta::assert_snapshot!(
//             render(|a, _| Diagnostic {
//                 severity: Severity::Error,
//                 message: "test message".into(),
//                 labels: vec![primary(a, 0, 5, "test label")],
//                 notes: Vec::new(),
//             }),
//             @"
//             error: test message
//               ╭─[test_a.txt:1:1]
//             1 │ lorem ipsum dolor sit amet consectetur adipiscing
//               · ═════ test label
//               │
//             ",
//         );
//     }

//     #[test]
//     fn single_line_label_with_note() {
//         insta::assert_snapshot!(
//             render(|a, _| Diagnostic {
//                 severity: Severity::Error,
//                 message: "test message".into(),
//                 labels: vec![primary(a, 0, 5, "test label")],
//                 notes: vec!["test note".into()],
//             }),
//             @"
//             error: test message
//               ╭─[test_a.txt:1:1]
//             1 │ lorem ipsum dolor sit amet consectetur adipiscing
//               · ═════ test label
//               │
//               = test note
//             ",
//         );
//     }

//     #[test]
//     fn multiple_labels_fit_on_one_line() {
//         insta::assert_snapshot!(
//             render(|a, _| Diagnostic {
//                 severity: Severity::Error,
//                 message: "test message".into(),
//                 labels: vec![
//                     primary(a, 0, 5, "primary label"),
//                     secondary(a, 22, 26, "secondary label"),
//                 ],
//                 notes: Vec::new(),
//             }),
//             @"
//             error: test message
//               ╭─[test_a.txt:1:1]
//             1 │ lorem ipsum dolor sit amet consectetur adipiscing
//               · ═════ primary label   ──── secondary label
//               │
//             ",
//         );
//     }

//     #[test]
//     fn multiple_labels_dont_fit_on_one_line() {
//         insta::assert_snapshot!(
//             render(|a, _| Diagnostic {
//                 severity: Severity::Error,
//                 message: "test message".into(),
//                 labels: vec![
//                     primary(a, 0, 5, "primary label"),
//                     secondary(a, 12, 17, "secondary label"),
//                 ],
//                 notes: Vec::new(),
//             }),
//             @"
//             error: test message
//               ╭─[test_a.txt:1:1]
//             1 │ lorem ipsum dolor sit amet consectetur adipiscing
//               · ╦════       ───── secondary label
//               · ╚═ primary label
//               │
//             ",
//         );
//     }

//     #[test]
//     fn primary_containing_secondary() {
//         insta::assert_snapshot!(
//             render(|a, _| Diagnostic {
//                 severity: Severity::Error,
//                 message: "test message".into(),
//                 labels: vec![
//                     primary(a, 0, 17, "primary label"),
//                     secondary(a, 6, 11, "secondary label"),
//                 ],
//                 notes: Vec::new(),
//             }),
//             @"
//             error: test message
//               ╭─[test_a.txt:1:1]
//             1 │ lorem ipsum dolor sit amet consectetur adipiscing
//               ·       ───── secondary label
//               · ═════════════════ primary label
//               │
//             ",
//         );
//     }

//     #[test]
//     fn secondary_containing_primary() {
//         insta::assert_snapshot!(
//             render(|a, _| Diagnostic {
//                 severity: Severity::Error,
//                 message: "test message".into(),
//                 labels: vec![
//                     primary(a, 6, 11, "primary label"),
//                     secondary(a, 0, 17, "secondary label"),
//                 ],
//                 notes: Vec::new(),
//             }),
//             @"
//             error: test message
//               ╭─[test_a.txt:1:7]
//             1 │ lorem ipsum dolor sit amet consectetur adipiscing
//               ·       ═════ primary label
//               · ───────────────── secondary label
//               │
//             ",
//         );
//     }

//     #[test]
//     fn layered_label_containment() {
//         insta::assert_snapshot!(
//             render(|a, _| Diagnostic {
//                 severity: Severity::Error,
//                 message: "test message".into(),
//                 labels: vec![
//                     primary(a, 23, 38, "primary label"),
//                     secondary(a, 6, 17, "label 2"),
//                     secondary(a, 27, 38, "label 3"),
//                     secondary(a, 0, 22, "label 4"),
//                     secondary(a, 0, 38, "label 5"),
//                 ],
//                 notes: Vec::new(),
//             }),
//             @"
//             error: test message
//               ╭─[test_a.txt:1:1]
//             1 │ lorem ipsum dolor sit amet consectetur adipiscing
//               ·       ─────────── label 2  ─────────── label 3
//               · ┬──────────────────── ════════════════ primary label
//               · └─ label 4
//               · ───────────────────────────────────────────────── label 5
//               │
//             ",
//         );
//     }
// }
