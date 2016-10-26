use std::char;

use track::*;
use token::{Token, TokenData, Exp, CharCase, Sign, NumberSource, Radix, StringLiteral, RegExpLiteral};
use word::Map as WordMap;

use std::cell::Cell;
use std::rc::Rc;
use context::Context;
use char::ESCharExt;
use reader::Reader;
use lookahead::Buffer;
use error::Error;
use result::Result;

fn add_digits(digits: Vec<u32>, radix: u32) -> u32 {
    let mut place = 1;
    let mut sum = 0;
    for digit in digits.iter().rev() {
        sum += digit * place;
        place *= radix;
    }
    sum
}

struct SpanTracker {
    start: Posn
}

impl SpanTracker {
    fn end<I>(&self, lexer: &Lexer<I>, value: TokenData) -> Token
      where I: Iterator<Item=char>
    {
        let end = lexer.posn();
        Token::new(self.start, end, value)
    }
}

pub struct Lexer<I> {
    reader: Reader<I>,
    cx: Rc<Cell<Context>>,
    lookahead: Buffer,
    wordmap: WordMap
}

impl<I> Lexer<I> where I: Iterator<Item=char> {
    // constructor

    pub fn new(chars: I, cx: Rc<Cell<Context>>) -> Lexer<I> {
        Lexer {
            reader: Reader::new(chars),
            cx: cx,
            lookahead: Buffer::new(),
            wordmap: WordMap::new()
        }
    }

    // public methods

    pub fn peek_token(&mut self) -> Result<&Token> {
        if self.lookahead.is_empty() {
            let token = try!(self.read_next_token());
            self.lookahead.push_token(token);
        }
        Ok(self.lookahead.peek_token())
    }

    pub fn repeek_token(&mut self) -> &Token {
        debug_assert!(!self.lookahead.is_empty());
        self.lookahead.peek_token()
    }

    pub fn skip_token(&mut self) -> Result<()> {
        try!(self.read_token());
        Ok(())
    }

    pub fn reread_token(&mut self) -> Token {
        debug_assert!(!self.lookahead.is_empty());
        self.lookahead.read_token()
    }

    pub fn read_token(&mut self) -> Result<Token> {
        if self.lookahead.is_empty() {
            self.read_next_token()
        } else {
            Ok(self.lookahead.read_token())
        }
    }

    pub fn unread_token(&mut self, token: Token) {
        self.lookahead.unread_token(token);
    }

    // source location

    pub fn posn(&self) -> Posn {
        self.reader.curr_posn()
    }

    fn start(&self) -> SpanTracker {
        SpanTracker { start: self.posn() }
    }

    // generic lexing utilities

    fn read(&mut self) -> char {
        let ch = self.reader.curr_char().unwrap();
        self.skip();
        ch
    }

    fn reread(&mut self, ch: char) -> char {
        debug_assert!(self.peek() == Some(ch));
        self.skip();
        ch
    }

    fn peek(&mut self) -> Option<char> {
        self.reader.curr_char()
    }

    fn peek2(&mut self) -> (Option<char>, Option<char>) {
        (self.reader.curr_char(), self.reader.next_char())
    }

    fn skip(&mut self) {
        self.reader.skip();
    }

    fn skip2(&mut self) {
        self.skip();
        self.skip();
    }

    fn matches(&mut self, ch: char) -> bool {
        (self.peek() == Some(ch)) && { self.skip(); true }
    }

    fn skip_while<F>(&mut self, pred: &F)
      where F: Fn(char) -> bool
    {
        self.skip_until(&|ch| !pred(ch))
    }

    fn skip_until<F>(&mut self, pred: &F)
      where F: Fn(char) -> bool
    {
        loop {
            match self.peek() {
                Some(ch) if pred(ch) => return,
                None => return,
                _ => ()
            }
            self.skip();
        }
    }

    fn read_into_until<F>(&mut self, s: &mut String, pred: &F)
      where F: Fn(char) -> bool
    {
        loop {
            match self.peek() {
                Some(ch) if pred(ch) => return,
                Some(ch) => { s.push(self.reread(ch)); }
                None => return,
            }
        }
    }

    fn read_into2_until<F>(&mut self, s1: &mut String, s2: &mut String, pred: &F)
      where F: Fn(char) -> bool
    {
        loop {
            match self.peek() {
                Some(ch) if pred(ch) => return,
                Some(ch) => {
                    self.reread(ch);
                    s1.push(ch);
                    s2.push(ch);
                }
                None => return,
            }
        }
    }

    fn read_until_with<F, G>(&mut self, pred: &F, read: &mut G) -> Result<()>
      where F: Fn(char) -> bool,
            G: FnMut(&mut Self) -> Result<()>
    {
        loop {
            match self.peek() {
                Some(ch) if pred(ch) => return Ok(()),
                Some(_) => { try!(read(self)); }
                None => return Ok(())
            }
        }
    }

    // lexical grammar

    fn skip_newlines(&mut self) {
        debug_assert!(self.peek().map_or(false, |ch| ch.is_es_newline()));
        loop {
            match self.peek2() {
                (Some('\r'), Some('\n'))            => { self.skip2(); }
                (Some(ch), _) if ch.is_es_newline() => { self.skip(); }
                _                                   => { break; }
            }
        }
    }

    fn read_newline_into(&mut self, s: &mut String) {
        debug_assert!(self.peek().map_or(false, |ch| ch.is_es_newline()));
        if self.peek2() == (Some('\r'), Some('\n')) {
            s.push_str("\r\n");
            self.skip2();
            return;
        }
        s.push(self.read());
    }

    fn skip_whitespace(&mut self) {
        self.skip_while(&|ch| ch.is_es_whitespace());
    }

    fn skip_line_comment(&mut self) {
        self.skip2();
        self.skip_until(&|ch| ch.is_es_newline());
    }

    fn skip_block_comment(&mut self) -> Result<bool> {
        self.skip2();
        let mut found_newline = false;
        loop {
            match self.peek2() {
                (None, _) | (_, None)  => { return Err(Error::UnterminatedComment); }
                (Some('*'), Some('/')) => { self.skip2(); break; }
                (Some(ch), _) => {
                    if ch.is_es_newline() {
                        found_newline = true;
                    }
                    self.skip();
                }
            }
        }
        Ok(found_newline)
    }

    fn read_regexp(&mut self) -> Result<Token> {
        let span = self.start();
        let mut s = String::new();
        self.reread('/');
        try!(self.read_until_with(&|ch| ch == '/', &mut |this| { this.read_regexp_char(&mut s) }));
        self.reread('/');
        let flags = try!(self.read_word_parts());
        Ok(span.end(self, TokenData::RegExp(RegExpLiteral {
            pattern: s,
            flags: flags.chars().collect()
        })))
    }

    fn read_regexp_char(&mut self, s: &mut String) -> Result<()> {
        match self.peek() {
            Some('\\') => self.read_regexp_backslash(s),
            Some('[') => self.read_regexp_class(s),
            Some(ch) if ch.is_es_newline() => Err(Error::UnterminatedRegExp(Some(ch))),
            Some(ch) => { s.push(self.reread(ch)); Ok(()) }
            None => Err(Error::UnterminatedRegExp(None))
        }
    }

    fn read_regexp_backslash(&mut self, s: &mut String) -> Result<()> {
        s.push(self.reread('\\'));
        match self.peek() {
            Some(ch) if ch.is_es_newline() => Err(Error::UnterminatedRegExp(Some(ch))),
            Some(ch) => { s.push(self.reread(ch)); Ok(()) }
            None => Err(Error::UnterminatedRegExp(None))
        }
    }

    fn read_regexp_class(&mut self, s: &mut String) -> Result<()> {
        s.push(self.reread('['));
        try!(self.read_until_with(&|ch| ch == ']', &mut |this| { this.read_regexp_class_char(s) }));
        s.push(self.reread(']'));
        Ok(())
    }

    fn read_regexp_class_char(&mut self, s: &mut String) -> Result<()> {
        match self.peek() {
            Some('\\') => self.read_regexp_backslash(s),
            Some(ch) => { s.push(self.reread(ch)); Ok(()) }
            None => Err(Error::UnterminatedRegExp(None))
        }
    }

    fn read_decimal_digits_into(&mut self, s: &mut String) {
        self.read_into_until(s, &|ch| !ch.is_digit(10));
    }

    fn read_decimal_digits(&mut self) -> String {
        let mut s = String::new();
        self.read_decimal_digits_into(&mut s);
        s
    }

    fn read_exp_part(&mut self) -> Result<Option<Exp>> {
        let e = match self.peek() {
            Some('e') => CharCase::LowerCase,
            Some('E') => CharCase::UpperCase,
            _ => { return Ok(None); }
        };
        self.skip();
        let sign = match self.peek() {
            Some('+') => { self.skip(); Some(Sign::Plus) }
            Some('-') => { self.skip(); Some(Sign::Minus) }
            _ => None
        };
        let mut value = String::new();
        match self.peek() {
            Some(ch) if !ch.is_digit(10) => return Err(Error::MissingExponent(Some(ch))),
            None => { return Err(Error::MissingExponent(None)); }
            _ => ()
        }
        self.read_decimal_digits_into(&mut value);
        Ok(Some(Exp { e: e, sign: sign, value: value }))
    }

    fn read_decimal_int(&mut self) -> String {
        let mut s = String::new();
        self.read_into_until(&mut s, &|ch| !ch.is_digit(10));
        s
    }

    fn read_radix_int<F, G>(&mut self, radix: u32, pred: &F, cons: &G, missing_digits: Error) -> Result<Token>
      where F: Fn(char) -> bool,
            G: Fn(CharCase, String) -> TokenData
    {
        debug_assert!(self.reader.curr_char() == Some('0'));
        debug_assert!(self.reader.next_char().map_or(false, |ch| ch.is_alphabetic()));
        let span = self.start();
        let mut s = String::new();
        self.skip();
        let flag = if self.read().is_lowercase() {
            CharCase::LowerCase
        } else {
            CharCase::UpperCase
        };
        try!(self.read_digit_into(&mut s, radix, pred, missing_digits));
        self.read_into_until(&mut s, &|ch| !pred(ch));
        Ok(span.end(self, cons(flag, s)))
    }

    fn read_hex_int(&mut self) -> Result<Token> {
        self.read_radix_int(16, &|ch| ch.is_es_hex_digit(), &|cc, s| {
            NumberSource::RadixInt(Radix::Hex(cc), s).into_token_data()
        }, Error::MissingHexDigits)
    }

    fn read_oct_int(&mut self) -> Result<Token> {
        self.read_radix_int(8, &|ch| ch.is_es_oct_digit(), &|cc, s| {
            NumberSource::RadixInt(Radix::Oct(Some(cc)), s).into_token_data()
        }, Error::MissingOctalDigits)
    }

    fn read_bin_int(&mut self) -> Result<Token> {
        self.read_radix_int(2, &|ch| ch.is_es_bin_digit(), &|cc, s| {
            NumberSource::RadixInt(Radix::Bin(cc), s).into_token_data()
        }, Error::MissingBinaryDigits)
    }

    fn read_deprecated_oct_int(&mut self) -> Token {
        let span = self.start();
        self.skip();
        let mut s = String::new();
        self.read_into_until(&mut s, &|ch| !ch.is_digit(10));
        span.end(self, if s.chars().all(|ch| ch.is_es_oct_digit()) {
            NumberSource::RadixInt(Radix::Oct(None), s).into_token_data()
        } else {
            NumberSource::DecimalInt(format!("0{}", s), None).into_token_data()
        })
    }

    fn read_number(&mut self) -> Result<Token> {
        let result = try!(match self.peek2() {
            (Some('0'), Some('x')) | (Some('0'), Some('X')) => self.read_hex_int(),
            (Some('0'), Some('o')) | (Some('0'), Some('O')) => self.read_oct_int(),
            (Some('0'), Some('b')) | (Some('0'), Some('B')) => self.read_bin_int(),
            (Some('0'), Some(ch)) if ch.is_digit(10) => Ok(self.read_deprecated_oct_int()),
            (Some('.'), _) => {
                let span = self.start();
                self.skip();
                let frac = self.read_decimal_digits();
                let exp = try!(self.read_exp_part());
                Ok(span.end(self, NumberSource::Float(None, Some(frac), exp).into_token_data()))
            }
            (Some(ch), _) => {
                debug_assert!(ch.is_digit(10));
                let span = self.start();
                let pos = self.read_decimal_int();
                let (dot, frac) = if self.matches('.') {
                    (true, Some(match self.peek() {
                        Some(ch) if ch.is_digit(10) => self.read_decimal_digits(),
                        _ => String::from("")
                    }))
                } else {
                    (false, None)
                };
                let exp = try!(self.read_exp_part());
                Ok(span.end(self, if dot {
                    NumberSource::Float(Some(pos), frac, exp).into_token_data()
                } else {
                    NumberSource::DecimalInt(pos, exp).into_token_data()
                }))
            }
            (None, _) => { panic!("read_number() called at EOF"); }
        });
        match self.peek() {
            Some(ch) if ch.is_es_identifier_start() => { return Err(Error::IdAfterNumber(ch)); }
            Some(ch) if ch.is_digit(10) => { return Err(Error::DigitAfterNumber(ch)); }
            _ => {}
        }
        Ok(result)
    }

    fn read_string(&mut self) -> Result<Token> {
        debug_assert!(self.peek().is_some());
        let span = self.start();
        let mut source = String::new();
        let mut value = String::new();
        let quote = self.read();
        source.push(quote);
        loop {
            self.read_into2_until(&mut source, &mut value, &|ch| {
                ch == quote ||
                ch == '\\' ||
                ch.is_es_newline()
            });
            match self.peek() {
                Some('\\') => {
                    try!(self.read_string_escape(&mut source, &mut value));
                }
                Some(ch) if ch.is_es_newline() => {
                    return Err(Error::UnterminatedString(Some(ch)));
                }
                Some(_) => {
                    source.push(quote);
                    self.skip();
                    break;
                }
                None => return Err(Error::UnterminatedString(None))
            }
        }
        Ok(span.end(self, TokenData::String(StringLiteral {
            source: Some(source),
            value: value
        })))
    }

    fn read_unicode_escape_seq(&mut self, s: &mut String) -> Result<u32> {
        if self.matches('{') {
            s.push('{');
            let mut digits = Vec::with_capacity(8);
            digits.push(try!(self.read_hex_digit_into(s)));
            try!(self.read_until_with(&|ch| ch == '}', &mut |this| {
                digits.push(try!(this.read_hex_digit_into(s)));
                Ok(())
            }));
            s.push(self.reread('}'));
            Ok(add_digits(digits, 16))
        } else {
            let mut place = 0x1000;
            let mut code_point = 0;
            for _ in 0..4 {
                code_point += try!(self.read_hex_digit_into(s)) * place;
                place >>= 4;
            }
            Ok(code_point)
        }
    }

    fn read_string_escape(&mut self, source: &mut String, value: &mut String) -> Result<()> {
        source.push(self.reread('\\'));
        match self.peek() {
            Some(ch) if ch.is_digit(8) => {
                let mut code = 0;
                for _ in 0..3 {
                    match self.peek() {
                        Some(ch) if ch.is_digit(8) => {
                            let new_code = (code << 3) + ch.to_digit(8).unwrap();
                            if new_code > 255 {
                                break;
                            }
                            source.push(self.reread(ch));
                            code = new_code;
                        },
                        _ => { break; }
                    }
                }
                value.push(char::from_u32(code).unwrap_or('?'));
            }
            Some(ch) if ch.is_es_single_escape_char() => {
                source.push(self.reread(ch));
                value.push(ch.unescape());
            }
            Some('x') => {
                source.push(self.reread('x'));
                let mut code = 0;
                code += try!(self.read_hex_digit_into(source)) << 4;
                code += try!(self.read_hex_digit_into(source));
                value.push(char::from_u32(code).unwrap_or('?'));
            }
            Some('u') => {
                source.push(self.reread('u'));
                let code = try!(self.read_unicode_escape_seq(source));
                value.push(char::from_u32(code).unwrap_or('?'));
            }
            Some(ch) if ch.is_es_newline() => {
                self.read_newline_into(source);
            }
            Some(ch) => {
                source.push(self.reread(ch));
                value.push(ch);
            }
            None => { } // error will be reported from caller
        }
        Ok(())
    }

    fn read_digit_into<F>(&mut self, s: &mut String, radix: u32, pred: &F, missing_digits: Error) -> Result<u32>
      where F: Fn(char) -> bool
    {
        match self.peek() {
            Some(ch) if pred(ch) => {
                s.push(self.reread(ch));
                debug_assert!(ch.is_digit(radix));
                Ok(ch.to_digit(radix).unwrap())
            },
            Some(ch) => Err(Error::InvalidDigit(ch)),
            None => Err(missing_digits)
        }
    }

    fn read_hex_digit_into(&mut self, s: &mut String) -> Result<u32> {
        self.read_digit_into(s, 16, &|ch| ch.is_es_hex_digit(), Error::MissingHexDigits)
    }

    fn read_word_parts(&mut self) -> Result<String> {
        let mut s = String::new();
        try!(self.read_until_with(&|ch| ch != '\\' && !ch.is_es_identifier_continue(), &mut |this| {
            match this.read() {
                '\\' => this.read_word_escape(&mut s),
                ch => { s.push(ch); Ok(()) }
            }
        }));
        Ok(s)
    }

    fn read_word(&mut self) -> Result<Token> {
        debug_assert!(self.peek().map_or(false, |ch| ch == '\\' || ch.is_es_identifier_start()));
        let span = self.start();
        let s = try!(self.read_word_parts());
        debug_assert!(s.len() > 0);
        Ok(span.end(self, self.wordmap.tokenize(s)))
    }

    fn read_word_escape(&mut self, s: &mut String) -> Result<()> {
        match self.peek() {
            Some('u') => { self.reread('u'); }
            cho => { return Err(Error::IncompleteWordEscape(cho)); }
        }
        let mut dummy = String::new();
        let code_point = try!(self.read_unicode_escape_seq(&mut dummy));
        match char::from_u32(code_point) {
            Some(ch) => { s.push(ch); Ok(()) }
            None => Err(Error::IllegalUnicode(code_point))
        }
    }

    fn read_punc(&mut self, value: TokenData) -> Token {
        let span = self.start();
        self.skip();
        span.end(self, value)
    }

    fn read_punc2(&mut self, value: TokenData) -> Token {
        let span = self.start();
        self.skip2();
        span.end(self, value)
    }

    fn read_punc2_3(&mut self, ch: char, value2: TokenData, value3: TokenData) -> Token {
        let span = self.start();
        self.skip2();
        let value = if self.matches(ch) { value3 } else { value2 };
        span.end(self, value)
    }

    fn read_next_token(&mut self) -> Result<Token> {
        let mut pair;
        let mut found_newline = false;

        // Skip whitespace and comments.
        loop {
            pair = self.peek2();
            match pair {
                (Some(ch), _) if ch.is_es_whitespace() => { self.skip_whitespace(); }
                (Some(ch), _) if ch.is_es_newline() => {
                    self.skip_newlines();
                    found_newline = true;
                }
                (Some('/'), Some('/')) => { self.skip_line_comment(); }
                (Some('/'), Some('*')) => {
                    found_newline = try!(self.skip_block_comment()) || found_newline;
                }
                _ => { break; }
            }
        }

        let mut result = try!(match pair {
            (Some('/'), _) if !self.cx.get().operator    => self.read_regexp(),
            (Some('/'), Some('='))                       => {
                Ok(self.read_punc2(TokenData::SlashAssign))
            }
            (Some('/'), _)                               => Ok(self.read_punc(TokenData::Slash)),
            (Some('.'), Some(ch)) if ch.is_digit(10)     => self.read_number(),
            (Some('.'), _)                               => Ok(self.read_punc(TokenData::Dot)),
            (Some('{'), _)                               => Ok(self.read_punc(TokenData::LBrace)),
            (Some('}'), _)                               => Ok(self.read_punc(TokenData::RBrace)),
            (Some('['), _)                               => Ok(self.read_punc(TokenData::LBrack)),
            (Some(']'), _)                               => Ok(self.read_punc(TokenData::RBrack)),
            (Some('('), _)                               => Ok(self.read_punc(TokenData::LParen)),
            (Some(')'), _)                               => Ok(self.read_punc(TokenData::RParen)),
            (Some(';'), _)                               => Ok(self.read_punc(TokenData::Semi)),
            (Some(':'), _)                               => Ok(self.read_punc(TokenData::Colon)),
            (Some(','), _)                               => Ok(self.read_punc(TokenData::Comma)),
            (Some('<'), Some('<'))                       => {
                Ok(self.read_punc2_3('=', TokenData::LShift, TokenData::LShiftAssign))
            }
            (Some('<'), Some('='))                       => Ok(self.read_punc2(TokenData::LEq)),
            (Some('<'), _)                               => Ok(self.read_punc(TokenData::LAngle)),
            (Some('>'), Some('>'))                       => Ok({
                let span = self.start();
                self.skip2();
                let value = match self.peek2() {
                    (Some('>'), Some('=')) => { self.skip2(); TokenData::URShiftAssign }
                    (Some('>'), _) => { self.skip(); TokenData::URShift }
                    (Some('='), _) => { self.skip(); TokenData::RShiftAssign }
                    _ => TokenData::RShift
                };
                span.end(self, value)
            }),
            (Some('>'), Some('='))                       => Ok(self.read_punc2(TokenData::GEq)),
            (Some('>'), _)                               => Ok(self.read_punc(TokenData::RAngle)),
            (Some('='), Some('='))                       => {
                Ok(self.read_punc2_3('=', TokenData::Eq, TokenData::StrictEq))
            }
            (Some('='), _)                               => Ok(self.read_punc(TokenData::Assign)),
            (Some('+'), Some('+'))                       => Ok(self.read_punc2(TokenData::Inc)),
            (Some('+'), Some('='))                       => {
                Ok(self.read_punc2(TokenData::PlusAssign))
            }
            (Some('+'), _)                               => Ok(self.read_punc(TokenData::Plus)),
            (Some('-'), Some('-'))                       => Ok(self.read_punc2(TokenData::Dec)),
            (Some('-'), Some('='))                       => {
                Ok(self.read_punc2(TokenData::MinusAssign))
            }
            (Some('-'), _)                               => Ok(self.read_punc(TokenData::Minus)),
            (Some('*'), Some('='))                       => {
                Ok(self.read_punc2(TokenData::StarAssign))
            }
            (Some('*'), _)                               => Ok(self.read_punc(TokenData::Star)),
            (Some('%'), Some('='))                       => {
                Ok(self.read_punc2(TokenData::ModAssign))
            }
            (Some('%'), _)                               => Ok(self.read_punc(TokenData::Mod)),
            (Some('^'), Some('='))                       => {
                Ok(self.read_punc2(TokenData::BitXorAssign))
            }
            (Some('^'), _)                               => Ok(self.read_punc(TokenData::BitXor)),
            (Some('&'), Some('&'))                       => {
                Ok(self.read_punc2(TokenData::LogicalAnd))
            }
            (Some('&'), Some('='))                       => {
                Ok(self.read_punc2(TokenData::BitAndAssign))
            }
            (Some('&'), _)                               => Ok(self.read_punc(TokenData::BitAnd)),
            (Some('|'), Some('|'))                       => {
                Ok(self.read_punc2(TokenData::LogicalOr))
            }
            (Some('|'), Some('='))                       => {
                Ok(self.read_punc2(TokenData::BitOrAssign))
            }
            (Some('|'), _)                               => Ok(self.read_punc(TokenData::BitOr)),
            (Some('~'), _)                               => Ok(self.read_punc(TokenData::Tilde)),
            (Some('!'), Some('='))                       => {
                Ok(self.read_punc2_3('=', TokenData::NEq, TokenData::StrictNEq))
            }
            (Some('!'), _)                               => Ok(self.read_punc(TokenData::Bang)),
            (Some('?'), _)                               => Ok(self.read_punc(TokenData::Question)),
            (Some('"'), _) | (Some('\''), _)             => self.read_string(),
            (Some(ch), _) if ch.is_digit(10)             => self.read_number(),
            (Some(ch), _) if ch.is_es_identifier_start() => self.read_word(),
            (Some('\\'), _)                              => self.read_word(),
            (Some(ch), _)                                => Err(Error::IllegalChar(ch)),
            (None, _)                                    => {
                let here = self.posn();
                Ok(Token::new(here, here, TokenData::EOF))
            }
        });
        result.newline = found_newline;
        Ok(result)
    }
}

impl<I> Iterator for Lexer<I> where I: Iterator<Item=char> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        match self.read_token() {
            Ok(Token { value: TokenData::EOF, .. }) => None,
            Ok(t) => Some(t),
            Err(_) => None
        }
    }
}

#[cfg(test)]
mod tests {

    use test::{deserialize_lexer_tests, LexerTest};
    use lexer::Lexer;
    use result::Result;
    use context::Context;
    use token::{Token, TokenData};
    use std::cell::Cell;
    use std::rc::Rc;
    use std;

    fn lex2(source: &String, context: Context) -> Result<(Token, Token)> {
        let chars = source.chars();
        let cx = Rc::new(Cell::new(context));
        let mut lexer = Lexer::new(chars, cx.clone());
        Ok((try!(lexer.read_token()), try!(lexer.read_token())))
    }

    fn assert_test2(source: &str, expected: &std::result::Result<TokenData, String>, expected_next: TokenData, actual: Result<(Token, Token)>) {
        match (expected, &actual) {
            (&Ok(ref expected), &Ok((Token { value: ref actual, .. }, Token { value: ref actual_next, .. }))) => {
                if expected != actual || &expected_next != actual_next {
                    println!("failed test: {:?}", source);
                }
                assert_eq!(expected, actual);
                assert_eq!(&expected_next, actual_next);
            }
            (&Ok(_), &Err(ref err)) => {
                println!("failed test: {:?}", source);
                panic!("unexpected lexer error: {}", err);
            }
            (&Err(_), &Ok(_)) => {
                println!("failed test: {:?}", source);
                panic!("unexpected token, expected error");
            }
            (&Err(_), &Err(_)) => { }
        }
    }

    #[test]
    pub fn go() {
        let tests = deserialize_lexer_tests(include_str!("../tests/unit.json"));
        for LexerTest { source, context, expected } in tests {
            assert_test2(&source[..], &expected, TokenData::EOF, lex2(&source, context));
            assert_test2(&source[..], &expected, TokenData::EOF, lex2(&format!("{} ", source), context));
            assert_test2(&source[..], &expected, TokenData::EOF, lex2(&format!(" {}", source), context));
            assert_test2(&source[..], &expected, TokenData::EOF, lex2(&format!(" {} ", source), context));
            assert_test2(&source[..], &expected, TokenData::Semi, lex2(&format!("{};", source), context));
        }
    }

}
