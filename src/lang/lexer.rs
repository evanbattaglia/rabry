use winnow::combinator::separated;
use winnow::token::take_while;
use winnow::token::take_till;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::terminated;
use winnow::combinator::preceded;
use winnow::PResult;
use winnow::Parser;
use winnow::{
    ascii::multispace1,
    combinator::repeat,
    combinator::peek,
    error::ErrMode,
    token::any,
};
use winnow::combinator::dispatch;

// TODO comments

#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    Comment,
    Identifier(&'a str), // including keywords
    IdentifierWithAliases(Vec<&'a str>),
    AtIdentifier(&'a str),
    String(String),
}

impl<'a> Token<'a> {
    pub fn is_identifier(&self, s: &str) -> bool {
        match self {
            Token::Identifier(id) => *id == s,
            _ => false,
        }
    }
}

// WTF
impl<'a, E: for<'b> winnow::error::ParserError<&'b [Token<'a>]>> winnow::Parser<&[Token<'a>], Token<'a>, E> for Token<'static> {
    #[inline(always)]
    fn parse_next(&mut self, input: &mut &[Token<'a>]) -> Result<Token<'a>, ErrMode<E>> {
        any.verify(|t| *t == self.clone()).parse_next(input)
    }
}

fn string_fragment(i: &mut &str) -> PResult<String> {
    alt((
      "\\\\".map(|_| "\\".to_owned()),
      take_till(1.., ['"', '\\']).verify(|s: &str| !s.is_empty()).map(|s: &str| s.to_owned()),
      "\\\"".map(|_| "\"".to_owned()),
     )).parse_next(i)
}

fn string_internals(i: &mut &str) -> PResult<String> {
    repeat(1.., string_fragment)
        .fold(|| String::new(), |mut acc, s| {
            acc.push_str(&s);
            acc
        }).parse_next(i)
}

fn string<'a> (i: &mut &'a str) -> PResult<Token<'a>> {
    let s = delimited("\"", string_internals, "\"").parse_next(i)?;
    Ok(Token::String(s))
}

fn identifier_str<'a>(i: &mut &'a str) -> PResult<&'a str> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-')
        .parse_next(i)
}

fn identifier_with_aliases<'a>(i: &mut &'a str) -> PResult<Vec<&'a str>> {
    separated(1.., identifier_str, ",").parse_next(i)
}

fn identifier_with_optional_aliases<'a>(i: &mut &'a str) -> PResult<Token<'a>> {
    identifier_with_aliases.parse_next(i).map(|id| {
        if id.len() == 1 {
            Token::Identifier(id.get(0).unwrap())
        } else {
            Token::IdentifierWithAliases(id)
        }
    })
}

fn at_identifier<'a>(i: &mut &'a str) -> PResult<Token<'a>> {
    let (_, id) = ("@", take_while(1.., |c: char| c.is_alphanumeric() || c == '-' || c == '_')).parse_next(i)?;
    Ok(Token::AtIdentifier(id))
}

fn comment<'a>(i: &mut &'a str) -> PResult<Token<'a>> {
    // TODO haven't tested this yet, and westill need a way to ignore comments
    // maybe there's a way we don't need the dummy token
    let (_, _) = ("#", take_while(1.., |c: char| c != '\n')).parse_next(i)?;
    Ok(Token::Comment)
}

fn token<'a>(i: &mut &'a str) -> PResult<Token<'a>> {
    dispatch! { peek(any);
        '"' => string,
        '(' => "(".value(Token::OpenParen),
        ')' => ")".value(Token::CloseParen),
        '{' => "{".value(Token::OpenBrace),
        '}' => "}".value(Token::CloseBrace),
        '@' => at_identifier,
        //'#' => comment.void().map(|_| None),
        _ => identifier_with_optional_aliases,
    }.parse_next(i)
}

fn optional_ignored_text<'a>(i: &mut &'a str) -> PResult<()> {
    repeat(0.., alt((comment.void(), multispace1.void()))).parse_next(i)
}

pub fn lex<'a>(i: &mut &'a str) -> PResult<Vec<Token<'a>>> {
    preceded(
        optional_ignored_text,
        repeat(1.., terminated(token, optional_ignored_text))
    ).parse_next(i)
}

//--- end lexer---
