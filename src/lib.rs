use crate::error::SplitError;
use crate::token::Token;
use crate::token_group::TokenGroup;
use crate::tokenizer::Tokenizer;

pub mod error;
mod ext;
#[cfg(test)]
mod test_data;
pub mod token;
pub mod token_group;
mod tokenizer;

fn prepare_token_groups(html: &str) -> Result<Vec<TokenGroup>, SplitError> {
    // Since most of the html text this splitter is supposed to split is markdown-like formatting
    // converted to html, there will be no root element. Most of the tags will be like
    // `<b>something</b>`, or at worst `Some text <b>something <i>italic</i></b> blah blah`.
    // So, instead of trying to stuff the max possible amount of text into a single chunk, we prefer
    // to put it into the next one. Apart from that, all links, or bold titles, and other whatnot
    // will be moved to the next chunk if they don't fit. I guess it's better for messengers
    // where you would not like to read split titles.
    let mut stack = vec![];
    let mut token_group = TokenGroup::default();
    let mut token_groups = vec![];

    for token in Tokenizer::new(html) {
        token_group.push(token);

        match token {
            Token::OpenTag(_, _) => stack.push(token),
            Token::CloseTag(_, _) => {
                stack.pop().ok_or(SplitError::UnbalancedToken(token))?;
            }
            _ => {}
        }

        if stack.is_empty() {
            token_groups.push(token_group);
            token_group = TokenGroup::default();
        }
    }

    if !stack.is_empty() {
        return Err(SplitError::UnbalancedToken(stack.pop().unwrap()));
    }

    Ok(token_groups)
}

#[cfg(test)]
fn serialize_token_groups(groups: &[TokenGroup]) -> String {
    let mut result = String::new();
    for group in groups {
        result.push_str(&group.to_string());
    }

    result
}

pub fn split<'a>(
    text: &'a str,
    max_chunk_size: usize,
    no_split: &[&str],
) -> Result<Vec<String>, SplitError<'a>> {
    // We'd like to get off without involving subdividing token groups itself.
    // If we can open a new chunk, we do it. If the token group is larger than max_chunk_size, only
    // then do we call subdivide on it.

    let mut chunks = vec![];
    let mut chunk = String::new();

    let mut has_exceeded = false;

    for tg in prepare_token_groups(text)? {
        if chunk.len() + tg.len <= max_chunk_size {
            chunk.push_str(&tg.to_string());
            continue;
        }

        if tg.len <= max_chunk_size {
            chunks.push(chunk);
            chunks.push(tg.to_string());
            chunk = String::new();
            continue;
        }

        if !chunk.is_empty() {
            chunks.push(chunk);
            chunk = String::new();
        }

        let tgs = match tg.subdivide(max_chunk_size, no_split) {
            Ok(tgs) => tgs,
            Err(SplitError::SubdividedExceedingTheLimit(tgs)) => {
                has_exceeded = true;
                tgs
            }
            Err(err) => return Err(err),
        };

        for tg in tgs {
            chunks.push(tg.to_string());
        }
    }

    if !chunk.is_empty() {
        chunks.push(chunk);
    }

    if has_exceeded {
        return Err(SplitError::SplitExceededTheLimit(chunks));
    }

    Ok(chunks)
}

#[cfg(test)]
fn clean(html: impl AsRef<str>) -> String {
    use ammonia::Builder;
    use std::collections::HashSet;

    Builder::new()
        .tags(HashSet::default())
        .strip_comments(false)
        .clean(html.as_ref())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_data::LONG_HTML;
    use testresult::TestResult;

    #[test]
    fn test_prepare_token_groups() -> TestResult {
        let mut reassembled = String::new();
        for group in prepare_token_groups(LONG_HTML)? {
            reassembled.push_str(&group.to_string());
        }

        assert_eq!(reassembled, LONG_HTML);

        Ok(())
    }

    #[test]
    fn test_split_plain_text() {
        let text = "This is a simple plain text without any HTML tags.";
        let max_chunk_size = 10;

        let result = split(text, max_chunk_size, &[]).unwrap();
        for chunk in &result {
            assert!(
                chunk.len() <= max_chunk_size,
                "Chunk exceeds max_chunk_size: {}",
                chunk
            );
        }
        let joined_text = result.join("");
        assert_eq!(joined_text, text);
    }

    #[test]
    fn test_split_html_text() -> TestResult {
        let result = split(LONG_HTML, 100, &[])?;
        for chunk in &result {
            assert!(
                chunk.len() <= 100,
                "Chunk exceeds max_chunk_size: {}",
                chunk
            );
        }

        let joined_text = clean(result.join(""));
        let clean_html = clean(LONG_HTML);
        assert_eq!(joined_text, clean_html);

        Ok(())
    }

    #[test]
    fn test_split_html_text_multiple() -> TestResult {
        let clean_html = clean(LONG_HTML);

        for chunk_size in 60..4096 {
            let result = split(LONG_HTML, chunk_size, &[]);

            // we can occasionally run into a situation when we can't split
            let chunks = match result {
                Ok(chunks) => chunks,
                Err(SplitError::SplitExceededTheLimit(chunks)) => chunks,
                err => err?,
            };

            let joined_text = clean(chunks.join(""));
            assert_eq!(joined_text, clean_html);
        }

        Ok(())
    }
}
