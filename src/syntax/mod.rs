mod ast;
mod grammar;
mod kind;
mod lexer;
mod parser;
mod tree;

#[salsa::tracked]
pub(crate) fn parse(
    db: &dyn crate::Db,
    file: crate::source::File,
) -> (tree::Tree, Vec<crate::diagnostic::Diagnostic>) {
    let mut parser = parser::Parser::new(file, file.text(db));
    grammar::root(&mut parser);
    parser.finish()
}
