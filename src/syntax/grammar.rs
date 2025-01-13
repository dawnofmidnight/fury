use super::{
    kind::{Kind, t},
    parser::{Parser, expected},
};

#[cfg(test)]
use core::fmt::Write as _;

pub(super) fn root(p: &mut Parser<'_>) {
    let m = p.open();
    p.eat_trivia();
    while !p.at_end() {
        match p.peek() {
            t![fn] => fn_(p),
            found => {
                expected!(p, found, fn ("an item"));
                p.bump();
            }
        }
    }
    p.close(m, Kind::Root);
}

test!(empty, "");
test!(multiple_items, "fn foo() = true;\nfn bar() = false;");
test!(missing_item, "awawa");

fn fn_(p: &mut Parser<'_>) {
    let m = p.open();
    p.expect(t![fn]);
    p.expect(t![ident]);
    param_list(p);
    if p.optional(t![->]) {
        type_expr(p);
    }
    p.expect(t![=]);
    expr(p);
    p.expect(t![;]);
    p.close(m, Kind::Fn);
}

test!(fn_minimal, "fn foo() = true;");
test!(fn_params, "fn foo(x: Bool) = true;");
test!(fn_return_type, "fn foo() -> Bool = true;");

fn param_list(p: &mut Parser<'_>) {
    let m = p.open();
    p.expect(t!['(']);
    while p.peek() != t![')'] && !p.at_end() {
        param(p);
    }
    p.expect(t![')']);
    p.close(m, Kind::ParamList);
}

fn param(p: &mut Parser<'_>) {
    let m = p.open();
    p.expect(t![ident]);
    p.expect(t![:]);
    type_expr(p);
    if p.peek() != t![')'] {
        p.expect(t![,]);
    }
    p.close(m, Kind::Param);
}

test!(params_empty, "fn foo() = true;");
test!(params_no_trailing, "fn foo(x: Int, y: Bool) = true;");
test!(params_with_trailing, "fn foo(x: Int, y: Bool,) = true;");
test!(params_missing_recovery, "fn foo(x: , y Bool) = true;");

fn type_expr(p: &mut Parser<'_>) {
    let m = p.open();
    p.expect(t![ident]);
    p.close(m, Kind::TypeExpr);
}

fn expr(p: &mut Parser<'_>) {
    expr_delimited(p);
}

fn expr_delimited(p: &mut Parser<'_>) {
    let m = p.open();
    match p.peek() {
        t![int] | t![bool] => {
            p.bump();
            p.close(m, Kind::ExprLiteral);
        }
        t![ident] => {
            p.bump();
            p.close(m, Kind::ExprName);
        }
        t!['{'] => {
            p.expect(t!['{']);
            expr(p);
            p.expect(t!['}']);
            p.close(m, Kind::ExprGroup);
        }
        found => {
            expected!(p, found, int, bool, ident("an expression"));
            if !p.at_end() {
                p.bump();
            }
            p.close(m, Kind::Error);
        }
    }
}

test!(bool_literal, "fn foo() = true;");
test!(int_literal, "fn foo() = -123;");
test!(ident_expr, "fn foo() = bar;");
test!(expr_group, "fn foo() = { bar };");

#[cfg(test)]
fn test_output(input: &str) -> String {
    let db = &crate::Database::default();
    let file = crate::source::File::new(db, "<test>".into(), input.to_owned());
    let (tree, diagnostics) = crate::syntax::parse(db, file);
    let mut writer = crate::terminal::diagnostic::StringWriter(String::new());
    tree.debug(&mut writer.0, input).unwrap();
    writeln!(&mut writer.0).unwrap();
    crate::terminal::diagnostic::emit(db, &diagnostics, &mut termcolor::NoColor::new(&mut writer))
        .unwrap();
    writer.0
}

macro_rules! test {
    ($name:ident, $input:literal) => {
        #[cfg(test)]
        ::paste::paste! {
            #[test]
            fn [< test_ $name >]() {
                ::insta::assert_snapshot!(test_output($input));
            }
        }
    };
}
use test;
