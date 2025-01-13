pub(crate) mod diagnostic;

#[must_use]
pub fn build(file: std::path::PathBuf) -> std::process::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_env(
            "
FURY_LOG",
        ))
        .init();
    let db = crate::Database::default();
    match build_inner(&db, file) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(diagnostics) => {
            assert!(
                diagnostic::emit(
                    &db,
                    &diagnostics,
                    &mut termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto),
                )
                .is_ok(),
                "failed to write diagnostics to standard error"
            );
            std::process::ExitCode::FAILURE
        }
    }
}

fn build_inner(
    db: &dyn crate::Db,
    file: std::path::PathBuf,
) -> Result<(), Vec<crate::diagnostic::Diagnostic>> {
    if !file.is_file() {
        return Err(vec![crate::diagnostic::Diagnostic::error(format!(
            "provided path `{}` is not a file",
            file.display()
        ))]);
    }
    let Ok(text) = std::fs::read_to_string(&file) else {
        return Err(vec![crate::diagnostic::Diagnostic::error(format!(
            "failed to read fury source file `{}`",
            file.display()
        ))]);
    };
    let source = crate::source::Source::new(db, vec![crate::source::File::new(db, file, text)]);
    let diagnostics = crate::compile(db, source);
    match diagnostics.is_empty() {
        true => Ok(()),
        false => Err(diagnostics),
    }
}
