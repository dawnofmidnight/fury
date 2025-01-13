pub(super) fn to_std_range(
    db: &dyn crate::Db,
    file: crate::source::File,
    range: &lsp_types::Range,
) -> std::ops::Range<usize> {
    let lsp_types::Range { start, end } = range;
    let (start, end) = (location_from_lsp_position(*start), location_from_lsp_position(*end));
    let (start, end) =
        (file.location_utf16_to_index(db, start), file.location_utf16_to_index(db, end));
    start.try_into().unwrap()..end.try_into().unwrap()
}

fn location_from_lsp_position(position: lsp_types::Position) -> crate::source::LocationUtf16 {
    crate::source::LocationUtf16 { line: position.line + 1, column: position.character + 1 }
}

pub(super) fn from_lsp_uri(uri: &lsp_types::Uri) -> &std::path::Path {
    assert!(
        uri.scheme().is_some_and(|s| s.eq_lowercase("file")),
        "URIs must have file scheme: {:?}",
        uri.scheme()
    );
    assert!(
        uri.authority().is_none_or(|a| a.as_str().is_empty()),
        "URIs must not have an authority: '{}'",
        uri.authority().unwrap()
    );
    assert!(
        uri.query().is_none_or(|q| q.as_str().is_empty()),
        "URIs must not have a query: {:?}",
        uri.query()
    );
    assert!(
        uri.fragment().is_none_or(|f| f.as_str().is_empty()),
        "URIs must not have a fragment: '{:?}'",
        uri.fragment()
    );
    std::path::Path::new(uri.path().as_str())
}

pub(super) fn to_lsp_diagnostic(
    db: &dyn crate::Db,
    diagnostic: &crate::diagnostic::Diagnostic,
) -> Option<(lsp_types::Uri, lsp_types::Diagnostic)> {
    let mut message = diagnostic.message.to_string();
    for note in &diagnostic.notes {
        message.push_str("\nnote: ");
        message.push_str(note);
    }

    let main_label = diagnostic.primary.as_ref().or(diagnostic.secondary.first())?;
    let related_information = (if diagnostic.primary.is_some() {
        &diagnostic.secondary
    } else {
        diagnostic.secondary.get(1..).unwrap_or(&[])
    })
    .iter()
    .map(|label| lsp_types::DiagnosticRelatedInformation {
        location: to_lsp_location(db, label.file, label.span),
        message: label.message.to_string(),
    })
    .collect::<Vec<_>>();

    Some((
        to_lsp_uri(main_label.file.path(db)),
        lsp_types::Diagnostic {
            range: to_lsp_range(db, main_label.file, main_label.span),
            severity: Some(to_lsp_severity(diagnostic.severity)),
            code: None,
            code_description: None,
            source: None,
            message,
            related_information: (!related_information.is_empty()).then_some(related_information),
            tags: None,
            data: None,
        },
    ))
}

fn to_lsp_severity(severity: crate::diagnostic::Severity) -> lsp_types::DiagnosticSeverity {
    match severity {
        crate::diagnostic::Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
        crate::diagnostic::Severity::Warning => lsp_types::DiagnosticSeverity::WARNING,
        crate::diagnostic::Severity::Note => lsp_types::DiagnosticSeverity::INFORMATION,
        crate::diagnostic::Severity::Help => lsp_types::DiagnosticSeverity::HINT,
    }
}

fn to_lsp_location(
    db: &dyn crate::Db,
    file: crate::source::File,
    span: crate::source::Span,
) -> lsp_types::Location {
    lsp_types::Location { uri: to_lsp_uri(file.path(db)), range: to_lsp_range(db, file, span) }
}

pub(super) fn to_lsp_uri(path: &std::path::Path) -> lsp_types::Uri {
    // all paths come from `uri_to_path` and are thus utf-8, so this is fine
    format!("file:///{}", path.display()).parse().unwrap()
}

fn to_lsp_range(
    db: &dyn crate::Db,
    file: crate::source::File,
    span: crate::source::Span,
) -> lsp_types::Range {
    lsp_types::Range {
        start: to_lsp_position(db, file, span.start),
        end: to_lsp_position(db, file, span.end),
    }
}

fn to_lsp_position(
    db: &dyn crate::Db,
    file: crate::source::File,
    index: u32,
) -> lsp_types::Position {
    let location = file.index_to_location_utf16(db, index);
    lsp_types::Position { line: location.line - 1, character: location.column - 1 }
}
