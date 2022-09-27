// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]
use crate::sema::ast::{Expression, Parameter, Statement, TryCatch, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::unescape;
use crate::sema::yul::ast::InlineAssembly;
use crate::{parse_and_resolve, sema::ast, FileResolver, Target};
use solang_parser::pt::Loc;
use std::ffi::OsStr;

pub(crate) fn parse(src: &'static str) -> ast::Namespace {
    let mut cache = FileResolver::new();
    cache.set_file_contents("test.sol", src.to_string());

    let ns = parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::EVM);
    ns.print_diagnostics_in_plain(&cache, false);
    ns
}

#[test]
fn test_unescape() {
    let s = r#"\u00f3"#;
    let mut vec = Diagnostics::default();
    let res = unescape(s, 0, 0, &mut vec);
    assert!(vec.is_empty());
    assert_eq!(res, vec![0xc3, 0xb3]);
    let s = r#"\xff"#;
    let res = unescape(s, 0, 0, &mut vec);
    assert!(vec.is_empty());
    assert_eq!(res, vec![255]);
}

#[test]
fn test_statement_reachable() {
    let loc = Loc::File(0, 1, 2);
    let test_cases: Vec<(Statement, bool)> = vec![
        (Statement::Underscore(loc), true),
        (
            Statement::Destructure(loc, vec![], Expression::BoolLiteral(loc, true)),
            true,
        ),
        (
            Statement::VariableDecl(
                loc,
                0,
                Parameter {
                    loc,
                    id: None,
                    ty: Type::Bool,
                    ty_loc: None,
                    indexed: false,
                    readonly: false,
                    recursive: false,
                },
                None,
            ),
            true,
        ),
        (
            Statement::Emit {
                loc,
                event_no: 0,
                event_loc: Loc::Builtin,
                args: vec![],
            },
            true,
        ),
        (
            Statement::Delete(loc, Type::Bool, Expression::BoolLiteral(loc, true)),
            true,
        ),
        (Statement::Continue(loc), false),
        (Statement::Break(loc), false),
        (Statement::Return(loc, None), false),
        (
            Statement::If(
                loc,
                false,
                Expression::BoolLiteral(loc, false),
                vec![],
                vec![],
            ),
            false,
        ),
        (
            Statement::While(loc, true, Expression::BoolLiteral(loc, false), vec![]),
            true,
        ),
        (
            Statement::DoWhile(loc, false, vec![], Expression::BoolLiteral(loc, true)),
            false,
        ),
        (
            Statement::Expression(loc, true, Expression::BoolLiteral(loc, false)),
            true,
        ),
        (
            Statement::For {
                loc,
                reachable: false,
                init: vec![],
                cond: None,
                next: vec![],
                body: vec![],
            },
            false,
        ),
        (
            Statement::TryCatch(
                loc,
                true,
                TryCatch {
                    expr: Expression::BoolLiteral(loc, false),
                    returns: vec![],
                    ok_stmt: vec![],
                    errors: vec![],
                    catch_param: None,
                    catch_param_pos: None,
                    catch_stmt: vec![],
                },
            ),
            true,
        ),
        (
            Statement::Assembly(
                InlineAssembly {
                    loc,
                    body: vec![],
                    functions: std::ops::Range { start: 0, end: 0 },
                },
                false,
            ),
            false,
        ),
    ];

    for (test_case, expected) in test_cases {
        assert_eq!(test_case.reachable(), expected);
    }
}

#[test]
fn constant_overflow_checks() {
    let file = r#"
    contract test_contract {
        function test_params(uint8 usesa, int8 sesa) public {}

        function test_add(int8 input) public returns (uint8) {
            // value 133 does not fit into type int8.
            int8 add_ovf = 127 + 6;

            // negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.
            uint8 negative = 3 - 4;

            // value 133 does not fit into type int8.
            int8 mixed = 126 + 7 + input;

            // negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.
            return 1 - 2;
        }

        function test_mul(int8 input) public {
            // value 726 does not fit into type int8.
            int8 mul_ovf = 127 * 6;

            // value 882 does not fit into type int8.
            int8 mixed = 126 * 7 * input;
        }

        function test_shift(int8 input) public {
            // value 128 does not fit into type int8.
            int8 mul_ovf = 1 << 7;

            // value 128 does not fit into type int8.
            int8 mixed = (1 << 7) + input;
        }

        function test_call() public {
            // negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.
            // value 129 does not fit into type int8.
            test_params(1 - 2, 127 + 2);

            // negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.
            // value 129 does not fit into type int8.
            test_params({usesa: 1 - 2, sesa: 127 + 2});
        }

        function test_builtin (bytes input) public{

            // value 4294967296 does not fit into type uint32.
            int16 sesa = input.readInt16LE(4294967296);
        }

        function test_for_loop () public {

            for (int8 i = 125 + 5; i < 300 ; i++) {

            }

        }
    }

        "#;
    let ns = parse(file);
    let errors = ns.diagnostics.errors();

    assert_eq!(errors[0].message, "value 133 does not fit into type int8.");
    assert_eq!(errors[1].message,"negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.");
    assert_eq!(errors[2].message, "value 133 does not fit into type int8.");
    assert_eq!(errors[4].message, "value 762 does not fit into type int8.");
    assert_eq!(errors[5].message, "value 882 does not fit into type int8.");
    assert_eq!(errors[6].message, "value 128 does not fit into type int8.");
    assert_eq!(errors[7].message, "value 128 does not fit into type int8.");
    assert_eq!(errors[8].message,"negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.");
    assert_eq!(errors[9].message, "value 129 does not fit into type int8.");
    assert_eq!(errors[10].message,"negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.");
    assert_eq!(errors[11].message, "value 129 does not fit into type int8.");
    assert_eq!(
        errors[12].message,
        "value 4294967296 does not fit into type uint32."
    );
    assert_eq!(errors[13].message, "value 130 does not fit into type int8.");
}
