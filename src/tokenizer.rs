use crate::token::Token;

#[derive(Debug)]
pub(crate) struct Tokenizer<'a> {
    text: &'a str,
    index: usize,
}

impl<'a> Tokenizer<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        Self { text, index: 0 }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let text = self.text;
        if text.is_empty() {
            return None;
        }

        let Some(open_pos) = text.find('<') else {
            let token = Token::Text(text, self.index);
            self.text = "";
            return Some(token);
        };

        if open_pos != 0 {
            let token = &text[..open_pos];
            self.text = &text[open_pos..];
            let token = Token::Text(token, self.index);
            self.index += open_pos;
            return Some(token);
        }

        let close_pos = text[open_pos + 1..]
            .find('>')
            .expect("missing close bracket")
            + open_pos
            + 1;

        let tag = &text[..close_pos + 1];

        let is_close = tag.chars().skip(1).find(|ch| !ch.is_whitespace()) == Some('/');
        let token = if is_close {
            Token::CloseTag(tag, self.index)
        } else {
            Token::OpenTag(tag, self.index)
        };
        self.index += close_pos + 1;

        self.text = &text[close_pos + 1..];
        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_and_close_tags() {
        let tokenizer = Tokenizer::new("<tag>content</tag>");
        let tokens: Vec<_> = tokenizer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::OpenTag("<tag>", 0),
                Token::Text("content", 5),
                Token::CloseTag("</tag>", 12)
            ]
        );
    }

    #[test]
    fn test_text_and_tags() {
        let tokenizer = Tokenizer::new("Text before\n<tag>Text inside</tag> Text after");
        let tokens: Vec<_> = tokenizer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::Text("Text before\n", 0),
                Token::OpenTag("<tag>", 12),
                Token::Text("Text inside", 17),
                Token::CloseTag("</tag>", 28),
                Token::Text(" Text after", 34),
            ]
        );
    }

    #[test]
    fn test_nested_tags() {
        let tokenizer = Tokenizer::new("<outer><inner>Content</inner></outer> ");
        let tokens: Vec<_> = tokenizer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::OpenTag("<outer>", 0),
                Token::OpenTag("<inner>", 7),
                Token::Text("Content", 14),
                Token::CloseTag("</inner>", 21),
                Token::CloseTag("</outer>", 29),
                Token::Text(" ", 37),
            ]
        );
    }

    #[test]
    fn test_values_with_slash() {
        let tokenizer = Tokenizer::new(r#"<a href="http://www.example.com/">inline URL</a>"#);
        let tokens: Vec<_> = tokenizer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::OpenTag(r#"<a href="http://www.example.com/">"#, 0),
                Token::Text("inline URL", 34),
                Token::CloseTag(r#"</a>"#, 44),
            ]
        )
    }

    #[test]
    fn test_self_closing_tag() {
        let tokenizer = Tokenizer::new("<img src='image.png'/>");
        let tokens: Vec<_> = tokenizer.collect();
        // TODO: introduce OpenClose support for the case + CDATA
        assert_eq!(tokens, vec![Token::OpenTag("<img src='image.png'/>", 0)]);
    }

    #[test]
    fn test_text_only() {
        let tokenizer = Tokenizer::new("Hello, world!");
        let tokens: Vec<_> = tokenizer.collect();
        assert_eq!(tokens, vec![Token::Text("Hello, world!", 0)]);
    }

    #[test]
    fn test_single_open_tag() {
        let tokenizer = Tokenizer::new("<tag>");
        let tokens: Vec<_> = tokenizer.collect();
        assert_eq!(tokens, vec![Token::OpenTag("<tag>", 0)]);
    }
}
