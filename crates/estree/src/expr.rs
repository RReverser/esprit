use serde_json::value::Value;
use easter::expr::Expr;
use easter::obj::DotKey;
use easter::id::IdExt;
use easter::punc::{Unop, Binop, Assop, Logop};
use unjson::ty::{Object, TyOf};
use unjson::ExtractField;
use joker::token::RegExpLiteral;

use tag::{Tag, TagOf};
use id::IntoId;
use result::Result;
use error::{Error, string_error, node_type_error, type_error};
use node::ExtractNode;
use fun::IntoFun;
use lit::{IntoStringLiteral, IntoNumberLiteral};

pub trait IntoExpr {
    fn into_expr(self) -> Result<Expr>;
    fn into_lit(self) -> Result<Expr>;
}

impl IntoExpr for Object {
    fn into_expr(mut self) -> Result<Expr> {
        let tag = try!(self.tag());
        Ok(match tag {
            Tag::Identifier => { return Ok(try!(self.into_id()).into_expr()); }
            Tag::Literal => try!(IntoExpr::into_lit(self)),
            Tag::BinaryExpression => {
                let str = try!(self.extract_string("operator").map_err(Error::Json));
                let op: Binop = match str.parse() {
                    Ok(op) => op,
                    Err(_) => { return string_error("binary operator", str); }
                };
                let left = try!(self.extract_expr("left"));
                let right = try!(self.extract_expr("right"));
                Expr::Binop(try!(self.extract_loc()), op, Box::new(left), Box::new(right))
            }
            Tag::AssignmentExpression => {
                let str = try!(self.extract_string("operator").map_err(Error::Json));
                let right = Box::new(try!(self.extract_expr("right")));
                match &str[..] {
                    "=" => Expr::Assign(try!(self.extract_loc()), try!(self.extract_assign_patt("left")), right),
                    _ => {
                        let op: Assop = match str.parse() {
                            Ok(op) => op,
                            Err(_) => { return string_error("assignment operator", str); }
                        };
                        Expr::BinAssign(try!(self.extract_loc()), op, try!(self.extract_assign_target("left")), right)
                    }
                }
            }
            Tag::LogicalExpression => {
                let str = try!(self.extract_string("operator").map_err(Error::Json));
                let op: Logop = match str.parse() {
                    Ok(op) => op,
                    Err(_) => { return string_error("logical operator", str); }
                };
                let left = try!(self.extract_expr("left"));
                let right = try!(self.extract_expr("right"));
                Expr::Logop(try!(self.extract_loc()), op, Box::new(left), Box::new(right))
            }
            Tag::UnaryExpression => {
                let str = try!(self.extract_string("operator").map_err(Error::Json));
                let op: Unop = match str.parse() {
                    Ok(op) => op,
                    Err(_) => { return string_error("unary operator", str); }
                };
                let arg = try!(self.extract_expr("argument"));
                Expr::Unop(try!(self.extract_loc()), op, Box::new(arg))
            }
            Tag::UpdateExpression => {
                let op = try!(self.extract_string("operator").map_err(Error::Json));
                let arg = Box::new(try!(self.extract_assign_target("argument")));
                let prefix = try!(self.extract_bool("prefix").map_err(Error::Json));
                match (&op[..], prefix) {
                    ("++", true)  => Expr::PreInc(try!(self.extract_loc()), arg),
                    ("++", false) => Expr::PostInc(try!(self.extract_loc()), arg),
                    ("--", true)  => Expr::PreDec(try!(self.extract_loc()), arg),
                    ("--", false) => Expr::PostDec(try!(self.extract_loc()), arg),
                    _ => { return string_error("'++' or '--'", op); }
                }
            }
            Tag::MemberExpression => {
                let obj = Box::new(try!(self.extract_expr("object")));
                if try!(self.extract_bool("computed").map_err(Error::Json)) {
                    let prop = Box::new(try!(self.extract_expr("property")));
                    Expr::Brack(try!(self.extract_loc()), obj, prop)
                } else {
                    let id = try!(try!(self.extract_object("property").map_err(Error::Json)).into_id());
                    let key = DotKey { location: id.location, value: id.name.into_string() };
                    Expr::Dot(try!(self.extract_loc()), obj, key)
                }
            }
            Tag::CallExpression => {
                let callee = Box::new(try!(self.extract_expr("callee")));
                let args = try!(self.extract_expr_list("arguments"));
                Expr::Call(try!(self.extract_loc()), callee, args)
            }
            Tag::NewExpression => {
                let callee = Box::new(try!(self.extract_expr("callee")));
                let args = try!(self.extract_expr_list("arguments"));
                Expr::New(try!(self.extract_loc()), callee, Some(args))
            }
            Tag::ArrayExpression => {
                let elts = try!(self.extract_expr_opt_list("elements"));
                Expr::Arr(try!(self.extract_loc()), elts)
            }
            Tag::FunctionExpression => {
                let fun = try!(self.into_fun());
                Expr::Fun(fun)
            }
            Tag::SequenceExpression => {
                let exprs = try!(self.extract_expr_list("expressions"));
                Expr::Seq(try!(self.extract_loc()), exprs)
            }
            Tag::ObjectExpression => {
                let props = try!(self.extract_prop_list("properties"));
                Expr::Obj(try!(self.extract_loc()), props)
            }
            Tag::ConditionalExpression => {
                let test = Box::new(try!(self.extract_expr("test")));
                let cons = Box::new(try!(self.extract_expr("consequent")));
                let alt = Box::new(try!(self.extract_expr("alternate")));
                Expr::Cond(try!(self.extract_loc()), test, cons, alt)
            }
            Tag::ThisExpression => Expr::This(try!(self.extract_loc())),
            _ => { return node_type_error("expression", tag); }
        })
    }

    fn into_lit(mut self) -> Result<Expr> {
        let json = try!(self.extract_field("value").map_err(Error::Json));
        let loc = try!(self.extract_loc());
        Ok(match json {
            Value::Null if !self.contains_key("regex") => Expr::Null(loc),
            Value::Bool(true) => Expr::True(loc),
            Value::Bool(false) => Expr::False(loc),
            Value::String(value) => Expr::String(loc, value.into_string_literal()),
            Value::I64(val) => Expr::Number(loc, val.into_number_literal()),
            Value::U64(val) => Expr::Number(loc, val.into_number_literal()),
            Value::F64(val) => Expr::Number(loc, val.into_number_literal()),
            Value::Null | Value::Object(_) => {
                let mut regex = try!(self.extract_object("regex").map_err(Error::Json));
                let pattern = try!(regex.extract_string("pattern").map_err(Error::Json));
                let flags = try!(regex.extract_string("flags").map_err(Error::Json));
                Expr::RegExp(loc, RegExpLiteral {
                    pattern: pattern,
                    flags: flags.chars().collect()
                })
            }
            _ => { return type_error("null, number, boolean, string, or object", json.ty()); }
        })
    }
}
