#![cfg(test)]

use std::collections::HashSet;
use serde_json;
use serde_json::Value;
use unjson::*;
use unjson::error::*;
use unjson::result::Result;
use unjson::ty::*;
use token::*;
use word::{Reserved, Name};
use std;

macro_rules! right {
    ( $l:expr, $r:expr ) => { $r }
}

macro_rules! tuplify {
    ( $v:expr, ( $($dummy:tt),* ) ) => {
        {
            let mut t = $v.into_iter();
            ($(
                right!($dummy, t.next().unwrap())
            ),*)
        }
    };
}

pub struct LexerTest {
    pub source: String,
    pub operator: bool,
    pub expected: std::result::Result<TokenData, String>
}

pub trait IntoTest {
    fn into_lexer_test(self) -> Result<LexerTest>;
}

pub trait IntoTestSuite {
    fn into_lexer_test_suite(self) -> Result<Vec<LexerTest>>;
}

impl IntoTestSuite for Array {
    fn into_lexer_test_suite(self) -> Result<Vec<LexerTest>> {
        self.into_iter()
        .map(|data| data.into_lexer_test())
        .collect()
    }
}

impl IntoTestSuite for Value {
    fn into_lexer_test_suite(self) -> Result<Vec<LexerTest>> {
        self.into_array().and_then(|arr| arr.into_lexer_test_suite())
    }
}

impl IntoTest for Object {
    fn into_lexer_test(mut self) -> Result<LexerTest> {
        let source = self.extract_string("source")?;
        let set = self.extract_array("context").and_then(|arr| arr.into_string_set())?;
        let expected = if self.contains_key("error") {
            Err(self.extract_string("error")?)
        } else {
            Ok(self.extract_field("expected").and_then(|data| data.into_token())?)
        };
        let operator = set.contains("operator");
        Ok(LexerTest {
            source: source,
            operator: operator,
            expected: expected
        })
    }
}

impl IntoTest for Value {
    fn into_lexer_test(self) -> Result<LexerTest> {
        self.into_object().and_then(|obj| obj.into_lexer_test())
    }
}

trait IntoStringSet {
    fn into_string_set(self) -> Result<HashSet<String>>;
}

impl IntoStringSet for Array {
    fn into_string_set(self) -> Result<HashSet<String>> {
        self.into_iter()
        .map(|data| data.into_string())
        .collect()
    }
}

pub trait IntoName {
    fn into_name(self) -> Result<Name>;
}

impl IntoName for Value {
    fn into_name(self) -> Result<Name> {
        Ok(Name::from(self.into_string()?))
    }
}

pub trait IntoReserved {
    fn into_reserved(self) -> Result<Reserved>;
}

impl IntoReserved for String {
    fn into_reserved(self) -> Result<Reserved> {
        Ok(match &self[..] {
            "Null"       => Reserved::Null,
            "True"       => Reserved::True,
            "False"      => Reserved::False,
            "Break"      => Reserved::Break,
            "Case"       => Reserved::Case,
            "Catch"      => Reserved::Catch,
            "Class"      => Reserved::Class,
            "Const"      => Reserved::Const,
            "Continue"   => Reserved::Continue,
            "Debugger"   => Reserved::Debugger,
            "Default"    => Reserved::Default,
            "Delete"     => Reserved::Delete,
            "Do"         => Reserved::Do,
            "Else"       => Reserved::Else,
            "Export"     => Reserved::Export,
            "Extends"    => Reserved::Extends,
            "Finally"    => Reserved::Finally,
            "For"        => Reserved::For,
            "Function"   => Reserved::Function,
            "If"         => Reserved::If,
            "Import"     => Reserved::Import,
            "In"         => Reserved::In,
            "Instanceof" => Reserved::Instanceof,
            "New"        => Reserved::New,
            "Return"     => Reserved::Return,
            "Super"      => Reserved::Super,
            "Switch"     => Reserved::Switch,
            "This"       => Reserved::This,
            "Throw"      => Reserved::Throw,
            "Try"        => Reserved::Try,
            "Typeof"     => Reserved::Typeof,
            "Var"        => Reserved::Var,
            "Void"       => Reserved::Void,
            "While"      => Reserved::While,
            "With"       => Reserved::With,
            "Enum"       => Reserved::Enum,
            _            => { return string_error("keyword", self); }
        })
    }
}

pub trait IntoToken {
    fn into_char_case(self) -> Result<CharCase>;
    fn into_char_case_opt(self) -> Result<Option<CharCase>>;
    fn into_exp_opt(self) -> Result<Option<Exp>>;
    fn into_token(self) -> Result<TokenData>;
}

fn validate_token(arr: Array) -> Result<Array> {
    if arr.len() == 0 {
        return index_error(0, arr.len());
    }

    let expected_len = {
        let elt = arr.iter().next().unwrap();
        let ty = match elt.as_string() {
            None      => { return type_error("string", elt.ty()); }
            Some(str) => str
        };
        match ty {
            "Reserved"   => 2,
            "DecimalInt" => 3,
            "BinaryInt"  => 3,
            "OctalInt"   => 3,
            "HexInt"     => 3,
            "Float"      => 4,
            "String"     => 2,
            "RegExp"     => 3,
            "Identifier" => 2,
            _            => 1
        }
    };

    let actual_len = arr.len();
    if actual_len != expected_len {
        return array_error(expected_len, actual_len);
    }

    Ok(arr)
}

impl IntoToken for Value {
    fn into_char_case(self) -> Result<CharCase> {
        let s = self.into_string()?;
        if s.len() == 0 {
            return string_error("lowercase or uppercase letter", s);
        }
        let ch = s.chars().next().unwrap();
        if ch.is_lowercase() {
            Ok(CharCase::LowerCase)
        } else if ch.is_uppercase() {
            Ok(CharCase::UpperCase)
        } else {
            string_error("lowercase or uppercase letter", s)
        }
    }

    fn into_char_case_opt(self) -> Result<Option<CharCase>> {
        match self {
            Value::Null => Ok(None),
            _ => self.into_char_case().map(Some)
        }
    }

    fn into_exp_opt(self) -> Result<Option<Exp>> {
        match self {
            Value::Null => Ok(None),
            _ => {
                let arr = self.into_array()?;
                if arr.len() != 3 {
                    return array_error(3, arr.len());
                }
                let (e, sign, value) = tuplify!(arr, ((), (), ()));
                Ok(Some(Exp {
                    e: e.into_char_case()?,
                    sign: match sign.into_string_opt()? {
                        None    => None,
                        Some(s) => {
                            if s.len() != 1 {
                                return string_error("'+' or '-'", s);
                            }
                            match s.chars().next().unwrap() {
                                '+' => Some(Sign::Plus),
                                '-' => Some(Sign::Minus),
                                _   => { return string_error("'+' or '-'", s); }
                            }
                        }
                    },
                    value: value.into_string()?
                }))
            }
        }
    }

    fn into_token(self) -> Result<TokenData> {
        let mut arr = self.into_array()?;

        // Check the array lengths in an external validation helper.
        // This lets us modularize the validation and avoids having to patch
        // the array back up to return in the error struct.
        arr = validate_token(arr)?;

        let ty = arr.remove(0).into_string()?;
        Ok(match &ty[..] {
            "Reserved"      => {
                let word = arr.remove(0).into_string().and_then(|str| str.into_reserved())?;
                TokenData::Reserved(word)
            }
            "LBrace"        => TokenData::LBrace,
            "RBrace"        => TokenData::RBrace,
            "LParen"        => TokenData::LParen,
            "RParen"        => TokenData::RParen,
            "LBrack"        => TokenData::LBrack,
            "RBrack"        => TokenData::RBrack,
            "Dot"           => TokenData::Dot,
            //"Ellipsis"    => TokenData::Ellipsis,
            "Semi"          => TokenData::Semi,
            "Comma"         => TokenData::Comma,
            "LAngle"        => TokenData::LAngle,
            "RAngle"        => TokenData::RAngle,
            "LEq"           => TokenData::LEq,
            "GEq"           => TokenData::GEq,
            "Eq"            => TokenData::Eq,
            "NEq"           => TokenData::NEq,
            "StrictEq"      => TokenData::StrictEq,
            "StrictNEq"     => TokenData::StrictNEq,
            "Plus"          => TokenData::Plus,
            "Minus"         => TokenData::Minus,
            "Star"          => TokenData::Star,
            "Mod"           => TokenData::Mod,
            "Slash"         => TokenData::Slash,
            "Inc"           => TokenData::Inc,
            "Dec"           => TokenData::Dec,
            "LShift"        => TokenData::LShift,
            "RShift"        => TokenData::RShift,
            "URShift"       => TokenData::URShift,
            "BitAnd"        => TokenData::BitAnd,
            "BitOr"         => TokenData::BitOr,
            "BitXor"        => TokenData::BitXor,
            "Bang"          => TokenData::Bang,
            "Tilde"         => TokenData::Tilde,
            "LogicalAnd"    => TokenData::LogicalAnd,
            "LogicalOr"     => TokenData::LogicalOr,
            "Question"      => TokenData::Question,
            "Colon"         => TokenData::Colon,
            "Assign"        => TokenData::Assign,
            "PlusAssign"    => TokenData::PlusAssign,
            "MinusAssign"   => TokenData::MinusAssign,
            "StarAssign"    => TokenData::StarAssign,
            "SlashAssign"   => TokenData::SlashAssign,
            "ModAssign"     => TokenData::ModAssign,
            "LShiftAssign"  => TokenData::LShiftAssign,
            "RShiftAssign"  => TokenData::RShiftAssign,
            "URShiftAssign" => TokenData::URShiftAssign,
            "BitAndAssign"  => TokenData::BitAndAssign,
            "BitOrAssign"   => TokenData::BitOrAssign,
            "BitXorAssign"  => TokenData::BitXorAssign,
            "Arrow"         => TokenData::Arrow,
            "EOF"           => TokenData::EOF,
            "DecimalInt"    => {
                let (value, exp) = tuplify!(arr, ((), ()));
                let value = value.into_string()?;
                let exp = exp.into_exp_opt()?;
                NumberSource::DecimalInt(value, exp).into_token_data()
            }
            "BinaryInt"     => {
                let (flag, value) = tuplify!(arr, ((), ()));
                let flag = flag.into_char_case()?;
                let value = value.into_string()?;
                NumberSource::RadixInt(Radix::Bin(flag), value).into_token_data()
            }
            "OctalInt"      => {
                let (flag, value) = tuplify!(arr, ((), ()));
                let flag = flag.into_char_case_opt()?;
                let value = value.into_string()?;
                NumberSource::RadixInt(Radix::Oct(flag), value).into_token_data()
            }
            "HexInt"        => {
                let (flag, value) = tuplify!(arr, ((), ()));
                let flag = flag.into_char_case()?;
                let value = value.into_string()?;
                NumberSource::RadixInt(Radix::Hex(flag), value).into_token_data()
            }
            "Float"         => {
                let (int, frac, exp) = tuplify!(arr, ((), (), ()));
                let int = int.into_string_opt()?;
                let frac = frac.into_string_opt()?;
                let exp = exp.into_exp_opt()?;
                NumberSource::Float(int, frac, exp).into_token_data()
            }
            "String"        => {
                let value = arr.remove(0).into_string()?;
                TokenData::String(StringLiteral {
                    source: Some(format!("{:?}", Value::String(value.to_string()))),
                    value: value
                })
            }
            "RegExp"        => {
                let (pattern, flags) = tuplify!(arr, ((), ()));
                let pattern = pattern.into_string()?;
                let flags = flags.into_string()?.chars().collect();
                TokenData::RegExp(RegExpLiteral {
                    pattern: pattern,
                    flags: flags
                })
            }
            "Identifier"    => TokenData::Identifier(arr.remove(0).into_name()?),
            _               => { return type_error("token", Ty::Array); }
        })
    }
}

pub fn deserialize_lexer_tests(src: &str) -> Vec<LexerTest> {
    let data: Value = match serde_json::from_str(src) {
        Ok(data) => data,
        Err(err) => panic!(format!("invalid JSON: {:?}", err))
    };
    match data.into_lexer_test_suite() {
        Ok(result) => result,
        Err(err) => panic!(format!("lexer test failed to deserialize: {}", err))
    }
}
