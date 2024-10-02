use crate::token::Token;
use crate::TokenGroup;
use std::fmt::Formatter;

#[derive(Debug)]
pub enum SplitError<'a> {
    SubdivisionImpossible(TokenGroup<'a>),
    SubdivisionImpossibleUnicode(Token<'a>),
    SubdividedExceedingTheLimit(Vec<TokenGroup<'a>>),
    SplitExceededTheLimit(Vec<String>),
    UnbalancedToken(Token<'a>),
    InvalidLen(usize),
}

impl std::fmt::Display for SplitError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SplitError::SubdivisionImpossible(tg) => {
                write!(f, "Subdivision impossible: {}", tg)
            }
            SplitError::UnbalancedToken(token) => {
                write!(f, "Unbalanced token: {}", token)
            }
            SplitError::InvalidLen(size) => {
                write!(f, "Invalid length: {}", size)
            }
            SplitError::SubdivisionImpossibleUnicode(token) => {
                write!(f, "Unicode subdivision impossible: {}", token)
            }
            SplitError::SubdividedExceedingTheLimit(token_groups) => {
                write!(f, "Exceeded the limit for {token_groups:?}")
            }
            SplitError::SplitExceededTheLimit(tgs) => {
                write!(f, "Split exceeded the limit for {tgs:?}")
            }
        }
    }
}
