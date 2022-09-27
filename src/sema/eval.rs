// SPDX-License-Identifier: Apache-2.0

use num_bigint::BigInt;
use num_bigint::Sign;
use num_rational::BigRational;
use num_traits::One;
use num_traits::ToPrimitive;
use num_traits::Zero;

use super::{
    ast::{Diagnostic, Expression, Namespace, Type},
    Recurse,
};
use solang_parser::pt;
use solang_parser::pt::{CodeLocation, Loc};
use std::ops::{Add, Mul, Shl, Shr, Sub};

/// Resolve an expression where a compile-time constant is expected
pub fn eval_const_number(
    expr: &Expression,
    ns: &Namespace,
) -> Result<(pt::Loc, BigInt), Diagnostic> {
    match expr {
        Expression::Add(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 + eval_const_number(r, ns)?.1,
        )),
        Expression::Subtract(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 - eval_const_number(r, ns)?.1,
        )),
        Expression::Multiply(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 * eval_const_number(r, ns)?.1,
        )),
        Expression::Divide(loc, _, l, r) => {
            let divisor = eval_const_number(r, ns)?.1;

            if divisor.is_zero() {
                Err(Diagnostic::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_number(l, ns)?.1 / divisor))
            }
        }
        Expression::Modulo(loc, _, l, r) => {
            let divisor = eval_const_number(r, ns)?.1;

            if divisor.is_zero() {
                Err(Diagnostic::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_number(l, ns)?.1 % divisor))
            }
        }
        Expression::BitwiseAnd(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 & eval_const_number(r, ns)?.1,
        )),
        Expression::BitwiseOr(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 | eval_const_number(r, ns)?.1,
        )),
        Expression::BitwiseXor(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 ^ eval_const_number(r, ns)?.1,
        )),
        Expression::Power(loc, _, _, base, exp) => {
            let b = eval_const_number(base, ns)?.1;
            let mut e = eval_const_number(exp, ns)?.1;

            if e.sign() == Sign::Minus {
                Err(Diagnostic::error(
                    expr.loc(),
                    "power cannot take negative number as exponent".to_string(),
                ))
            } else if e.sign() == Sign::NoSign {
                Ok((*loc, BigInt::one()))
            } else {
                let mut res = b.clone();
                e -= BigInt::one();
                while e.sign() == Sign::Plus {
                    res *= b.clone();
                    e -= BigInt::one();
                }
                Ok((*loc, res))
            }
        }
        Expression::ShiftLeft(loc, _, left, right) => {
            let l = eval_const_number(left, ns)?.1;
            let r = eval_const_number(right, ns)?.1;
            let r = match r.to_usize() {
                Some(r) => r,
                None => {
                    return Err(Diagnostic::error(
                        expr.loc(),
                        format!("cannot left shift by {}", r),
                    ));
                }
            };
            Ok((*loc, l << r))
        }
        Expression::ShiftRight(loc, _, left, right, _) => {
            let l = eval_const_number(left, ns)?.1;
            let r = eval_const_number(right, ns)?.1;
            let r = match r.to_usize() {
                Some(r) => r,
                None => {
                    return Err(Diagnostic::error(
                        expr.loc(),
                        format!("cannot right shift by {}", r),
                    ));
                }
            };
            Ok((*loc, l >> r))
        }
        Expression::NumberLiteral(loc, _, n) => Ok((*loc, n.clone())),
        Expression::ZeroExt(loc, _, n) => Ok((*loc, eval_const_number(n, ns)?.1)),
        Expression::SignExt(loc, _, n) => Ok((*loc, eval_const_number(n, ns)?.1)),
        Expression::Cast(loc, _, n) => Ok((*loc, eval_const_number(n, ns)?.1)),
        Expression::Not(loc, n) => Ok((*loc, !eval_const_number(n, ns)?.1)),
        Expression::Complement(loc, _, n) => Ok((*loc, !eval_const_number(n, ns)?.1)),
        Expression::UnaryMinus(loc, _, n) => Ok((*loc, -eval_const_number(n, ns)?.1)),
        Expression::ConstantVariable(_, _, Some(contract_no), var_no) => {
            let expr = ns.contracts[*contract_no].variables[*var_no]
                .initializer
                .as_ref()
                .unwrap()
                .clone();

            eval_const_number(&expr, ns)
        }
        Expression::ConstantVariable(_, _, None, var_no) => {
            let expr = ns.constants[*var_no].initializer.as_ref().unwrap().clone();

            eval_const_number(&expr, ns)
        }
        _ => Err(Diagnostic::error(
            expr.loc(),
            "expression not allowed in constant number expression".to_string(),
        )),
    }
}

/// Resolve an expression where a compile-time constant(rational) is expected
pub fn eval_const_rational(
    expr: &Expression,
    ns: &Namespace,
) -> Result<(pt::Loc, BigRational), Diagnostic> {
    match expr {
        Expression::Add(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_rational(l, ns)?.1 + eval_const_rational(r, ns)?.1,
        )),
        Expression::Subtract(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_rational(l, ns)?.1 - eval_const_rational(r, ns)?.1,
        )),
        Expression::Multiply(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_rational(l, ns)?.1 * eval_const_rational(r, ns)?.1,
        )),
        Expression::Divide(loc, _, l, r) => {
            let divisor = eval_const_rational(r, ns)?.1;

            if divisor.is_zero() {
                Err(Diagnostic::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_rational(l, ns)?.1 / divisor))
            }
        }
        Expression::Modulo(loc, _, l, r) => {
            let divisor = eval_const_rational(r, ns)?.1;

            if divisor.is_zero() {
                Err(Diagnostic::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_rational(l, ns)?.1 % divisor))
            }
        }
        Expression::NumberLiteral(loc, _, n) => Ok((*loc, BigRational::from_integer(n.clone()))),
        Expression::RationalNumberLiteral(loc, _, n) => Ok((*loc, n.clone())),
        Expression::Cast(loc, _, n) => Ok((*loc, eval_const_rational(n, ns)?.1)),
        Expression::UnaryMinus(loc, _, n) => Ok((*loc, -eval_const_rational(n, ns)?.1)),
        Expression::ConstantVariable(_, _, Some(contract_no), var_no) => {
            let expr = ns.contracts[*contract_no].variables[*var_no]
                .initializer
                .as_ref()
                .unwrap()
                .clone();

            eval_const_rational(&expr, ns)
        }
        Expression::ConstantVariable(_, _, None, var_no) => {
            let expr = ns.constants[*var_no].initializer.as_ref().unwrap().clone();

            eval_const_rational(&expr, ns)
        }
        _ => Err(Diagnostic::error(
            expr.loc(),
            "expression not allowed in constant rational number expression".to_string(),
        )),
    }
}

fn eval_constants_in_expression(
    expr: &Expression,
    ns: &Namespace,
    results: &mut Vec<(Type, BigInt)>,
) -> Option<Expression> {
    match expr {
        Expression::Add(loc, ty, _, left, right) => {
            let left = eval_constants_in_expression(left, ns, results);
            let right = eval_constants_in_expression(right, ns, results);

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                results.pop();
                results.pop();
                results.push((ty.clone(), left.add(right)));
                Some(Expression::NumberLiteral(*loc, ty.clone(), left.add(right)))
            } else {
                None
            }
        }
        Expression::Subtract(loc, ty, _, left, right) => {
            let left = eval_constants_in_expression(left, ns, results);
            let right = eval_constants_in_expression(right, ns, results);

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                results.pop();
                results.pop();
                results.push((ty.clone(), left.sub(right)));
                Some(Expression::NumberLiteral(*loc, ty.clone(), left.sub(right)))
            } else {
                None
            }
        }

        Expression::Multiply(loc, ty, _, left, right) => {
            let left = eval_constants_in_expression(left, ns, results);
            let right = eval_constants_in_expression(right, ns, results);

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                results.pop();
                results.pop();
                results.push((ty.clone(), left.mul(right.to_u32().unwrap())));
                Some(Expression::NumberLiteral(
                    *loc,
                    ty.clone(),
                    left.mul(right.to_u32().unwrap()),
                ))
            } else {
                None
            }
        }

        Expression::Power(loc, ty, _, left, right) => {
            let left = eval_constants_in_expression(left, ns, results);
            let right = eval_constants_in_expression(right, ns, results);

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                results.pop();
                results.pop();
                results.push((ty.clone(), left.pow(right.to_u32().unwrap())));
                Some(Expression::NumberLiteral(
                    *loc,
                    ty.clone(),
                    left.pow(right.to_u32().unwrap()),
                ))
            } else {
                None
            }
        }

        Expression::ShiftLeft(loc, ty, left, right) => {
            let left = eval_constants_in_expression(left, ns, results);
            let right = eval_constants_in_expression(right, ns, results);

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                results.pop();
                results.pop();
                results.push((ty.clone(), left.shl(right.to_u32().unwrap())));
                Some(Expression::NumberLiteral(
                    *loc,
                    ty.clone(),
                    left.shl(right.to_u32().unwrap()),
                ))
            } else {
                None
            }
        }

        Expression::ShiftRight(loc, ty, left, right, _) => {
            let left = eval_constants_in_expression(left, ns, results);
            let right = eval_constants_in_expression(right, ns, results);

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                results.pop();
                results.pop();
                results.push((ty.clone(), left.shr(right.to_u32().unwrap())));
                Some(Expression::NumberLiteral(
                    *loc,
                    ty.clone(),
                    left.shr(right.to_u32().unwrap()),
                ))
            } else {
                None
            }
        }
        Expression::NumberLiteral(_, ty, val) => {
            results.push((ty.clone(), val.clone()));
            Some(expr.clone())
        }
        _ => None,
    }
}

fn overflow_check(ns: &mut Namespace, result: BigInt, ty: Type, loc: Loc) {
    if let Type::Uint(bits) = ty {
        // If the result sign is minus, throw an error.
        if let Sign::Minus = result.sign() {
            ns.diagnostics.push(Diagnostic::error(
                loc,
            format!( "negative value {} does not fit into type {}. Cannot implicitly convert signed literal to unsigned type.",result,ty.to_string(ns)),
            ));
        }

        // If bits of the result is more than bits of the type, throw and error.
        if result.bits() > bits as u64 {
            ns.diagnostics.push(Diagnostic::error(
                loc,
                format!(
                    "value {} does not fit into type {}.",
                    result,
                    ty.to_string(ns)
                ),
            ));
        }
    }

    if let Type::Int(bits) = ty {
        // If number of bits is more than what the type can hold. BigInt.bits() is not used here since it disregards the sign.
        if result.to_signed_bytes_be().len() * 8 > (bits as usize) {
            ns.diagnostics.push(Diagnostic::error(
                loc,
                format!(
                    "value {} does not fit into type {}.",
                    result,
                    ty.to_string(ns)
                ),
            ));
        }
    }
}

pub fn verify_result(expr: &Expression, ns: &mut Namespace, loc: &Loc) {
    let mut results = Vec::new();
    expr.recurse(&mut results, |expr, results| {
        eval_constants_in_expression(expr, ns, results).is_none()
    });
    for (ty, result) in results {
        overflow_check(ns, result, ty, *loc);
    }
}
