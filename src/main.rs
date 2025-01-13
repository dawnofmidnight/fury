fn main() -> std::process::ExitCode {
    let cli = argh::from_env::<Cli>();

    match cli.command {
        Command::Build(args) => fury::terminal::build(args.file),
        Command::Lsp(Lsp {}) => fury::lsp::run(),
    }
}

/// the Fury programming language
#[derive(Debug, argh::FromArgs)]
struct Cli {
    #[argh(subcommand)]
    command: Command,
}

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand)]
enum Command {
    Build(Build),
    Lsp(Lsp),
}

/// build a Fury project
#[derive(Debug, argh::FromArgs)]
#[argh(subcommand, name = "build")]
struct Build {
    /// the Fury file to compile
    #[argh(positional)]
    file: std::path::PathBuf,
}

/// launch the Fury language server

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand, name = "lsp")]
struct Lsp {}
