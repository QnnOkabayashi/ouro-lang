use logos::{Lexer, Logos};
use ouro_span::Byte;
use ouro_tokenize_types::{Spans, TokenImpl, Tokenize};

// Moves the lexer to the end of the string or the end of the line, whichever comes first.
fn tokenize_string(lexer: &mut Lexer<'_, LogosToken>) -> Result<(), Error> {
    let remainder = lexer.remainder();

    let mut escaped = false;
    for (offset, byte) in remainder.bytes().enumerate() {
        if byte == b'\n' {
            lexer.bump(offset);
            return Err(Error::UnterminatedStr);
        }

        if escaped {
            escaped = false;
            continue;
        }

        if byte == b'"' {
            lexer.bump(offset + 1);
            return Ok(());
        }

        if byte == b'\\' {
            escaped = true;
        }
    }

    lexer.bump(remainder.len());
    Err(Error::UnterminatedStr)
}

fn advance_to_end_of_line(lexer: &mut Lexer<'_, LogosToken>) {
    let remainder = lexer.remainder();

    for (offset, byte) in remainder.bytes().enumerate() {
        if byte == b'\n' {
            lexer.bump(offset);
            return;
        }
    }

    lexer.bump(remainder.len());
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
enum Error {
    UnterminatedStr,
    #[default]
    Unrecognized,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Logos)]
#[logos(error = Error)]
enum LogosToken {
    /// Integers, or anything that might look like one.
    #[regex("[0-9][a-zA-Z0-9_]*", priority = 3)]
    IntLit,
    /// Identifiers.
    #[regex(r"\w+")]
    Ident,
    /// Strings like `"hello world"`.
    #[token("\"", tokenize_string)]
    StrLit,
    /// Fully escaped raw string.
    #[token("\\\\", advance_to_end_of_line)]
    RawStrLit,

    // Symbols
    #[token("{")]
    OpenBrace,
    #[token("}")]
    CloseBrace,
    #[token("(")]
    OpenParen,
    #[token(")")]
    CloseParen,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token("+")]
    Plus,
    #[token("-")]
    Dash,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("=")]
    Eq,
    #[token("==")]
    EqEq,
    #[token("->")]
    RArrow,
    #[token("!")]
    Bang,
    #[token("@")]
    Ampersand,

    // Keywords
    #[token("pub")]
    Pub,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("let")]
    Let,
    #[token("const")]
    Const,
    #[token("fn")]
    Fn,
    #[token("struct")]
    Struct,
    #[token("comptime")]
    Comptime,
    #[token("if")]
    If,
    #[token("else")]
    Else,

    // Builtin values
    #[token("i32")]
    I32Keyword,
    #[token("type")]
    TypeKeyword,

    // Other
    #[token("//", advance_to_end_of_line)]
    Comment,
    #[token("\n")]
    Newline,
    #[regex(r"[ \t\r\f]+")]
    Whitespace,
}

fn to_token_impl(token: Result<LogosToken, Error>) -> TokenImpl {
    match token {
        Ok(LogosToken::IntLit) => TokenImpl::IntLit,
        Ok(LogosToken::Ident) => TokenImpl::Ident,
        Ok(LogosToken::StrLit) => TokenImpl::StrLit,
        Ok(LogosToken::RawStrLit) => TokenImpl::RawStrLit,
        Ok(LogosToken::OpenBrace) => TokenImpl::OpenBrace,
        Ok(LogosToken::CloseBrace) => TokenImpl::CloseBrace,
        Ok(LogosToken::OpenParen) => TokenImpl::OpenParen,
        Ok(LogosToken::CloseParen) => TokenImpl::CloseParen,
        Ok(LogosToken::Dot) => TokenImpl::Dot,
        Ok(LogosToken::Comma) => TokenImpl::Comma,
        Ok(LogosToken::Colon) => TokenImpl::Colon,
        Ok(LogosToken::Semi) => TokenImpl::Semi,
        Ok(LogosToken::Plus) => TokenImpl::Plus,
        Ok(LogosToken::Dash) => TokenImpl::Dash,
        Ok(LogosToken::Star) => TokenImpl::Star,
        Ok(LogosToken::Slash) => TokenImpl::Slash,
        Ok(LogosToken::Eq) => TokenImpl::Eq,
        Ok(LogosToken::EqEq) => TokenImpl::EqEq,
        Ok(LogosToken::RArrow) => TokenImpl::RArrow,
        Ok(LogosToken::Bang) => TokenImpl::Bang,
        Ok(LogosToken::Ampersand) => TokenImpl::Ampersand,
        Ok(LogosToken::Pub) => TokenImpl::Pub,
        Ok(LogosToken::True) => TokenImpl::True,
        Ok(LogosToken::False) => TokenImpl::False,
        Ok(LogosToken::Let) => TokenImpl::Let,
        Ok(LogosToken::Const) => TokenImpl::Const,
        Ok(LogosToken::Fn) => TokenImpl::Fn,
        Ok(LogosToken::Struct) => TokenImpl::Struct,
        Ok(LogosToken::Comptime) => TokenImpl::Comptime,
        Ok(LogosToken::If) => TokenImpl::If,
        Ok(LogosToken::Else) => TokenImpl::Else,
        Ok(LogosToken::I32Keyword) => TokenImpl::I32Keyword,
        Ok(LogosToken::TypeKeyword) => TokenImpl::TypeKeyword,
        Ok(LogosToken::Comment) => TokenImpl::Comment,
        Ok(LogosToken::Newline) => TokenImpl::Newline,
        Ok(LogosToken::Whitespace) => TokenImpl::Whitespace,
        Err(Error::UnterminatedStr) => TokenImpl::UnterminatedStrLit,
        Err(Error::Unrecognized) => TokenImpl::Unrecognized,
    }
}

pub fn tokenize(input: &str) -> Tokenize {
    let (tokens, ends) = LogosToken::lexer(input)
        .spanned()
        .map(|(token, span)| (to_token_impl(token), Byte::from_usize(span.end)))
        .unzip();

    Tokenize {
        tokens,
        spans: Spans { ends },
    }
}
