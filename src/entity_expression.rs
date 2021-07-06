use nom::{IResult, named, map_res, tuple, opt, tag, alt, delimited, recognize, one_of, many0, pair,
          character::complete::multispace0 as ws, is_not, number::complete::double, error::Error,
          preceded, separated_list0, separated_pair, is_a, complete};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub enum Expression {
    Const(Const),
    Atom(String),
    BinOp(BinOp, Box<(Expression, Expression)>),
    UnOp(UnOp, Box<Expression>),
    Match { arms: HashMap<Const, Expression>, default: Box<Expression> }
}

#[derive(Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug)]
pub enum UnOp {
    Neg,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum Const {
    Number(Number),
    String(String),
}

#[derive(Debug)]
pub struct Number(f64);

// aggressively terrible solution to the floating point comparison problem:
// force all nan == nan, use normal fp comparison for others
impl PartialEq for Number {
    fn eq(&self, other: &Number) -> bool {
        if self.0.is_nan() && other.0.is_nan() {
            true
        } else {
            self.0 == other.0
        }
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.0.is_nan() {
            "nan".hash(state);
        } else {
            self.0.to_bits().hash(state);
        }
    }
}

named!(num_lit<&str, Number>,
    map_res!(
        alt!(double | hex_num),
        |s: f64| -> Result<Number, Error<&str>> { Ok(Number(s)) }
    )
);

named!(hex_num<&str, f64>,
    map_res!(
        tuple!(
            opt!(alt!(tag!("-") | tag!("+"))),
            alt!(tag!("0x") | tag!("0X")),
            nom::character::complete::hex_digit1
        ),
        |t: (Option<&str>, &str, &str)| i64::from_str_radix(t.2, 16).map(
            |q: i64| q as f64 * if t.0 == Some("-") { -1f64 } else { 1f64 }
        )
    )
);

// TODO use escaped_transform
named!(string_lit_dquote<&str, &str>,
    delimited!(tag!("\""),is_not!("\""),tag!("\""))
);

named!(string_lit_squote<&str, &str>,
    delimited!(tag!("'"),is_not!("'"),tag!("'"))
);

named!(string_lit<&str, &str>,
    alt!(string_lit_dquote | string_lit_squote)
);

const IDENT_START_CHARS: &str = "_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const IDENT_CONT_CHARS: &str = "_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

named!(atom<&str, Expression>,
    map_res!(
        delimited!(
            ws,
            recognize!(pair!(
                one_of!(IDENT_START_CHARS),
                many0!(complete!(one_of!(IDENT_CONT_CHARS)))
            )),
            ws),
        |s: &str| -> Result<Expression, Error<&str>> { Ok(Expression::Atom(s.to_owned())) }
    )
);

named!(expression<&str, Expression>,
    complete!(expression_3)
);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_atom() {
        println!("{:?}", expression("8"));
        assert_eq!(1, 2)
    }
}

named!(string_const<&str, Expression>,
    map_res!(
        delimited!(ws, string_lit, ws),
        |s: &str| -> Result<Expression, Error<&str>> { Ok(Expression::Const(Const::String(s.to_owned()))) }
    )
);

named!(num_const<&str, Expression>,
    map_res!(
        delimited!(ws, num_lit, ws),
        |s: Number| -> Result<Expression, Error<&str>> { Ok(Expression::Const(Const::Number(s))) }
    )
);

named!(parenthetical<&str, Expression>,
    delimited!(
        delimited!(ws, tag!("("), ws),
        expression_3,
        delimited!(ws, tag!(")"), ws)
    )
);

named!(match_case_const<&str, Option<Const>>,
    map_res!(
        alt!(string_const | num_const),
        |s: Expression| -> Result<Option<Const>, Error<&str>> { Ok(Some(match s { // oops I did not architect this correctly
            Expression::Const(c) => c,
            _ => unreachable!()
        })) }
    )
);

named!(match_case_default<&str, Option<Const>>,
    map_res!(
        delimited!(ws, tag!("_"), ws),
        |_| -> Result<Option<Const>, Error<&str>> { Ok(None) }
    )
);

named!(match_case<&str, Option<Const>>,
    alt!(match_case_const | match_case_default)
);

named!(match_arm<&str, (Option<Const>, Expression)>,
    separated_pair!(match_case, delimited!(ws, tag!("=>"), ws), expression_3)
);

named!(match_expr<&str, Expression>,
    preceded!(
        delimited!(ws, tag!("match"), ws),
        delimited!(
            delimited!(ws, tag!("{"), ws),
            map_res!(
                separated_list0!(delimited!(ws, tag!(","), ws), match_arm),
                |s: Vec<(Option<Const>, Expression)>| -> Result<Expression, ()> {
                    let mut arms: HashMap<Const, Expression> = HashMap::new();
                    let mut default: Option<Expression> = None;
                    for (case, expr) in s {
                        match case {
                            Some(real_case) => {
                                if arms.contains_key(&real_case) {
                                    return Err(())
                                }
                                arms.insert(real_case, expr);
                            },
                            None => {
                                if default.is_some() {
                                    return Err(())
                                }
                                default = Some(expr);
                            }
                        }
                    }

                    if default.is_none() {
                        return Err(())
                    }

                    Ok(Expression::Match {arms, default: Box::new(default.unwrap())})
                }
            ),
            delimited!(ws, tag!("}"), ws)
        )
    )
);

named!(expression_3<&str, Expression>,
    alt!(
        expression_2 |
        map_res!(
            tuple!(
                expression_3,
                delimited!(ws, alt!(tag!("+") | tag!("-")), ws),
                expression_2
            ), |s: (Expression, &str, Expression)| -> Result<Expression, Error<&str>> {
                Ok(Expression::BinOp(match s.1 {
                    "+" => BinOp::Add,
                    "-" => BinOp::Sub,
                    _ => unreachable!(),
                }, Box::new((s.0, s.2))))
            }
        )
    )
);

named!(expression_2<&str, Expression>,
    alt!(
        expression_1 |
        map_res!(
            tuple!(
                expression_2,
                delimited!(ws, alt!(tag!("*") | tag!("/") | tag!("%")), ws),
                expression_1
            ), |s: (Expression, &str, Expression)| -> Result<Expression, Error<&str>> {
                Ok(Expression::BinOp(match s.1 {
                    "*" => BinOp::Mul,
                    "/" => BinOp::Div,
                    "%" => BinOp::Mod,
                    _ => unreachable!(),
                }, Box::new((s.0, s.2))))
            }
        )
    )
);

named!(expression_1<&str, Expression>,
    alt!(
        expression_0 |
        map_res!(
            tuple!(
                delimited!(ws, tag!("-"), ws),
                expression_1
            ), |s: (&str, Expression)| -> Result<Expression, Error<&str>> {
                Ok(Expression::UnOp(match s.0 {
                    "-" => UnOp::Neg,
                    _ => unreachable!(),
                }, Box::new(s.1)))
            }
        )
    )
);

named!(expression_0<&str, Expression>,
    alt!(string_const | num_const | atom | parenthetical | match_expr)
);

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let parsed = expression(s.as_str());
        if let Err(e) = parsed {
            dbg!(e);
            panic!(); // ummmm how do you construct this kind of error
        }
        Ok(parsed.unwrap().1)
    }
}

impl Serialize for Expression {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        "can't serialize stuff yet!".serialize(s)
    }
}

impl Expression {
    pub fn mk_const(con: i32) -> Expression {
        Expression::Const(Const::Number(Number(con as f64)))
    }
}