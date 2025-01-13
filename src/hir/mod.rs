// use crate::source::Symbol;

// #[salsa::tracked]
// struct Item<'db> {
//     name: Symbol,
//     params: Vec<(Symbol, Type)>,
//     return_: Type,
//     body: Expr,
// }

// #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
// enum Type {
//     Bool,
//     Int,
// }

// #[derive(Clone, Debug, PartialEq, Eq, Hash)]
// enum Expr {
//     BoolLiteral(bool),
//     IntLiteral(Symbol),
// }

#[salsa::tracked]
pub fn check(
    db: &dyn crate::Db,
    source: crate::source::Source,
) -> Vec<crate::diagnostic::Diagnostic> {
    let mut diagnostics = Vec::new();
    for &file in source.files(db) {
        let (_, parse_diagnostics) = crate::syntax::parse(db, file);
        diagnostics.extend(parse_diagnostics);
    }
    diagnostics
}
