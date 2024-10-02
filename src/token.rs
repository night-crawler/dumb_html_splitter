use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum Token<'a> {
    OpenTag(&'a str, usize),
    CloseTag(&'a str, usize),
    Text(&'a str, usize),
}

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_text())
    }
}

impl<'a> Token<'a> {
    pub(crate) fn as_text(&self) -> &'a str {
        match self {
            Token::OpenTag(text, _) | Token::CloseTag(text, _) | Token::Text(text, _) => text,
        }
    }

    pub(crate) fn matches_tag_name(&self, tag: &str) -> bool {
        self.tag_name() == tag
    }

    pub(crate) fn len_since(&self, start: &Self) -> usize {
        self.index() + self.len() - start.index()
    }
    pub(crate) fn tag_name(&self) -> &str {
        match self {
            Token::OpenTag(text, _) | Token::CloseTag(text, _) => text
                .trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .trim()
                .trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap(),
            _ => "",
        }
    }

    pub(crate) fn is_close(&self) -> bool {
        matches!(self, Token::CloseTag(_, _))
    }

    pub(crate) fn is_open(&self) -> bool {
        matches!(self, Token::OpenTag(_, _))
    }

    pub(crate) fn len(&self) -> usize {
        self.as_text().len()
    }

    pub(crate) fn index(&self) -> usize {
        match self {
            Token::OpenTag(_, index) | Token::CloseTag(_, index) | Token::Text(_, index) => *index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_close_true() {
        let token = Token::CloseTag("</div>", 0);
        assert!(token.is_close());
    }

    #[test]
    fn test_is_close_false() {
        let token = Token::OpenTag("<div>", 0);
        assert!(!token.is_close());
    }

    #[test]
    fn test_is_close_self_closing() {
        let token = Token::OpenTag("<br/>", 0);
        assert!(!token.is_close()); // May need to adjust implementation
    }

    #[test]
    fn test_tag_name_simple_open() {
        let token = Token::OpenTag("<div>", 0);
        assert_eq!(token.tag_name(), "div");
    }

    #[test]
    fn test_tag_name_simple_close() {
        let token = Token::CloseTag("</div>", 0);
        assert_eq!(token.tag_name(), "div");
    }

    #[test]
    fn test_tag_name_with_attributes() {
        let token = Token::OpenTag("<div class='main'>", 0);
        assert_eq!(token.tag_name(), "div");
    }

    #[test]
    fn test_tag_name_self_closing() {
        let token = Token::OpenTag("<br/>", 0);
        assert_eq!(token.tag_name(), "br/");
    }

    #[test]
    #[should_panic]
    fn test_tag_name_empty_tag() {
        let token = Token::OpenTag("<>", 0);
        assert_eq!(token.tag_name(), "");
    }

    #[test]
    fn test_tag_name_malformed_tag() {
        let token = Token::OpenTag("<div", 0);
        assert_eq!(token.tag_name(), "div");
    }

    #[test]
    fn test_matches_tag_name_true() {
        let token = Token::OpenTag("<div>", 0);
        assert!(token.matches_tag_name("div"));
    }

    #[test]
    fn test_matches_tag_name_false() {
        let token = Token::OpenTag("<span>", 0);
        assert!(!token.matches_tag_name("div"));
    }

    #[test]
    fn test_matches_tag_name_self_closing() {
        let token = Token::OpenTag("<br/>", 0);
        assert!(token.matches_tag_name("br/"));
    }

    #[test]
    fn test_matches_tag_name_with_attributes() {
        let token = Token::OpenTag("<div class='main'>", 0);
        assert!(token.matches_tag_name("div"));
    }

    #[test]
    fn test_get_len_from_same_token() {
        let token = Token::Text("Hello", 0);
        assert_eq!(token.len_since(&token), token.len());
    }

    #[test]
    fn test_get_len_from_different_tokens() {
        let start_token = Token::Text("Hello", 10);
        let end_token = Token::Text("World", 100);
        assert_eq!(end_token.len_since(&start_token), 95);
    }
}
