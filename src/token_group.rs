use crate::error::SplitError;
use crate::ext::SplitPosExt;
use crate::token::Token;
use crate::tokenizer::Tokenizer;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Range;

#[derive(Debug, Default)]
pub struct TokenGroup<'a> {
    pub tokens: Vec<Token<'a>>,
    pub len: usize,
}

/// Root-level group of tokens
impl<'a> TokenGroup<'a> {
    pub(crate) fn push(&mut self, token: Token<'a>) {
        debug_assert!(token.len() != 0, "{token:?} has invalid length");

        let is_empty_tag =
            token.is_close() && self.tokens.last().map_or(false, |last| last.is_open());
        if is_empty_tag {
            self.pop();
            return;
        }

        self.tokens.push(token);
        self.len += token.len();
    }

    pub(crate) fn pop(&mut self) -> Option<Token<'a>> {
        let token = self.tokens.pop()?;
        self.len -= token.len();
        Some(token)
    }

    fn prepare_open_close_map(&self) -> Result<HashMap<Token<'a>, Token<'a>>, SplitError<'a>> {
        let mut map = HashMap::new();
        let mut stack = Vec::new();

        for token in self.tokens.iter().copied() {
            match token {
                Token::OpenTag(_, _) => {
                    stack.push(token);
                }
                Token::CloseTag(_, _) => {
                    let open = stack.pop().ok_or(SplitError::UnbalancedToken(token))?;
                    map.entry(open).or_insert(token);
                }
                Token::Text(_, _) => {}
            }
        }

        Ok(map)
    }

    fn get_close_token_index(
        &self,
        open_token_index: usize,
        map: &HashMap<Token<'a>, Token<'a>>,
    ) -> Result<usize, SplitError<'a>> {
        let open_token = self.tokens[open_token_index];
        let close_token = map[&open_token];

        self.tokens
            .iter()
            .copied()
            .enumerate()
            .skip(open_token_index)
            .find(|(_, tg)| tg == &close_token)
            .map(|(index, _)| index)
            .ok_or(SplitError::UnbalancedToken(close_token))
    }

    fn wrap(
        &self,
        range: Range<usize>,
        stack: &[Token<'a>],
        map: &HashMap<Token<'a>, Token<'a>>,
    ) -> Self {
        let mut tg = Self::default();
        tg.open_from_stack(stack);
        for token in self.tokens[range].iter().copied() {
            tg.push(token);
        }
        tg.close_from_stack(stack, map);
        tg
    }

    fn close_from_stack(&mut self, stack: &[Token<'a>], map: &HashMap<Token<'a>, Token<'a>>) {
        for token in stack.iter().map(|token| map[token]).rev() {
            self.push(token);
        }
    }

    fn open_from_stack(&mut self, stack: &[Token<'a>]) {
        for token in stack.iter().copied() {
            self.push(token);
        }
    }

    fn new_from_stack(stack: &[Token<'a>]) -> Self {
        let mut tg = TokenGroup::default();
        tg.open_from_stack(stack);
        tg
    }

    fn is_all_open(&self) -> bool {
        self.tokens.iter().all(Token::is_open)
    }

    // lifetime mismatch for the FromStr trait
    #[allow(clippy::should_implement_trait)]
    pub fn from_string(html: &'a str) -> Self {
        let mut tg = Self::default();
        for token in Tokenizer::new(html) {
            tg.push(token);
        }
        tg
    }
}

impl<'a> TokenGroup<'a> {
    pub fn subdivide(
        &self,
        max_chunk_size: usize,
        no_split: &[&str],
    ) -> Result<Vec<TokenGroup<'a>>, SplitError<'a>> {
        if max_chunk_size == 0 {
            return Err(SplitError::InvalidLen(max_chunk_size));
        }

        let map = self.prepare_open_close_map()?;
        let mut stack = vec![];
        let mut future_close_len = 0;
        let mut token_groups = vec![];
        let mut tg = TokenGroup::default();

        let mut index = 0;
        while index < self.tokens.len() {
            let token = self.tokens[index];
            let close_token = map.get(&token);
            let close_token_len = close_token.map(|token| token.len());

            let len_till_close = close_token.map(|ct| ct.len_since(&token));

            match token {
                // since we haven't opened the tag yet, we are free to stop right here
                Token::OpenTag(_, _) => {
                    let close_token = close_token.unwrap();
                    let close_token_len =
                        close_token_len.ok_or(SplitError::UnbalancedToken(*close_token))?;

                    // We look ahead for the close tag and check if it will need to be subdivided.
                    // In this case, we just immediately open a new token group despite the fact
                    // it still might not fit in max_chunk_size even after subdivision:
                    // we're doing our best, but if a no_split tag is too large, we can't fix it.
                    if no_split.contains(&token.tag_name())
                        && tg.len + future_close_len + len_till_close.unwrap() > max_chunk_size
                    {
                        let close_token_index = self.get_close_token_index(index, &map)?;
                        tg.close_from_stack(&stack, &map);
                        token_groups.push(tg);
                        tg = self.wrap(index..close_token_index + 1, &stack, &map);

                        // if we see that we are already exceeding the limit,
                        // recreate the token group
                        if tg.len + future_close_len >= max_chunk_size {
                            token_groups.push(tg);
                            tg = Self::new_from_stack(&stack);
                        }

                        // rewind to the position right after the close token
                        index = close_token_index + 1;
                        continue;
                    }

                    // Now, we solve the case when we know that there will be not enough space to
                    // close the currently open tags if we push this one
                    if tg.len + token.len() + close_token_len + future_close_len >= max_chunk_size {
                        // If all tags we added to the current group are open tags, and we've
                        // already run out of space, then there's no point in trying
                        if tg.is_all_open() {
                            return Err(SplitError::SubdivisionImpossible(tg));
                        }
                        tg.close_from_stack(&stack, &map);
                        token_groups.push(tg);
                        // we just need to clone the stack
                        tg = Self::new_from_stack(&stack);
                    }

                    future_close_len += close_token_len;
                    tg.push(token);
                    debug_assert!(tg.len <= max_chunk_size);
                    stack.push(token);
                    index += 1;
                }
                // since we have accounted for close tags when we opened them, we should not run
                // into a problem of splitting the mid close tag
                Token::CloseTag(_, _) => {
                    tg.push(token);
                    debug_assert!(tg.len <= max_chunk_size);

                    future_close_len -= token.len();
                    stack.pop().ok_or(SplitError::UnbalancedToken(token))?;
                    index += 1;
                }
                Token::Text(mut text, mut text_start_index) => {
                    let future_len = tg.len + future_close_len + token.len();
                    if future_len <= max_chunk_size {
                        tg.push(token);
                        assert!(tg.len <= max_chunk_size);

                        index += 1;
                        continue;
                    }

                    // Here we split the text till the first whitespace as long as it does not fit,
                    // and progress tag text + index
                    loop {
                        debug_assert!(tg.len <= max_chunk_size);
                        if future_close_len + tg.len > max_chunk_size {
                            return Err(SplitError::SubdivisionImpossible(tg));
                        }

                        let mut available_len = max_chunk_size - future_close_len - tg.len;
                        if available_len == 0 {
                            tg.close_from_stack(&stack, &map);
                            token_groups.push(tg);
                            tg = Self::new_from_stack(&stack);
                            available_len = max_chunk_size - future_close_len - tg.len;
                            if available_len == 0 {
                                return Err(SplitError::SubdivisionImpossible(tg));
                            }
                        }
                        let can_fit_segment = text
                            .split_with_respect_to_whitespace(available_len)
                            .ok_or(SplitError::SubdivisionImpossibleUnicode(token))?;

                        debug_assert!(!can_fit_segment.is_empty(), "{text}");
                        debug_assert!(
                            can_fit_segment.len() <= available_len,
                            "`{text}` got split into `{can_fit_segment}`; available_len: {available_len}"
                        );

                        tg.push(Token::Text(can_fit_segment, text_start_index));
                        debug_assert!(tg.len <= max_chunk_size);

                        text = &text[can_fit_segment.len()..];
                        text_start_index += can_fit_segment.len();

                        debug_assert!(!tg.is_all_open());
                        tg.close_from_stack(&stack, &map);
                        token_groups.push(tg);
                        tg = Self::new_from_stack(&stack);

                        if text.is_empty() {
                            break;
                        }
                    }

                    index += 1;
                }
            }
        }

        if !stack.is_empty() {
            return Err(SplitError::UnbalancedToken(stack.pop().unwrap()));
        }

        debug_assert!(tg.len <= max_chunk_size);
        debug_assert_eq!(future_close_len, 0);

        if !tg.tokens.is_empty() && !tg.is_all_open() {
            token_groups.push(tg);
        }

        // A case when we have no_split tags exceeding the max_chunk_size limit
        for tg in &token_groups {
            if tg.len > max_chunk_size {
                return Err(SplitError::SubdividedExceedingTheLimit(token_groups));
            }
        }
        Ok(token_groups)
    }
}

impl<'a> Display for TokenGroup<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for token in &self.tokens {
            write!(f, "{}", token)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_data::{LONG_HTML, SHORT_HTML};
    use crate::{clean, prepare_token_groups, serialize_token_groups};
    use testresult::TestResult;

    #[test]
    fn test_unicode_subdivide() -> TestResult {
        let html = r#"<tg-emoji emoji-id="5368324170671202286">👍</tg-emoji>"#;
        let tg = TokenGroup::from_string(html);

        for chunk_size in 0..56 {
            assert!(tg.subdivide(chunk_size, &[]).is_err())
        }

        let tgs = tg.subdivide(56, &[])?;
        assert_eq!(tgs.len(), 1);
        assert_eq!(tgs[0].len, html.len());

        Ok(())
    }

    #[test]
    fn test_subdivide_long() -> TestResult {
        assert!(prepare_token_groups(LONG_HTML).is_ok());

        let text = clean(LONG_HTML);
        let tg = TokenGroup::from_string(LONG_HTML);

        let subdivided = tg.subdivide(100, &["a"])?;
        assert_eq!(clean(serialize_token_groups(&subdivided)), text);

        Ok(())
    }

    #[test]
    fn test_prepare_token_groups() -> TestResult {
        let text = clean(SHORT_HTML);

        let mut token_groups = prepare_token_groups(SHORT_HTML)?;
        assert_eq!(token_groups.len(), 1);
        let token_group = token_groups.pop().unwrap();

        // <b><i><s><span class="tg-spoiler"></span></s></i></b> takes 53
        for chunk_size in 0..54 {
            let subdivided = token_group.subdivide(chunk_size, &["a"]);
            assert!(subdivided.is_err());
        }

        // must be ok for any range starting with 54
        for chunk_size in 54..4096 {
            let subdivided = token_group.subdivide(chunk_size, &["a"]);
            assert!(
                subdivided.is_ok(),
                "Failed subdivision for chunk size {chunk_size}"
            );
            let subdivided = subdivided?;
            assert_eq!(
                clean(serialize_token_groups(&subdivided)),
                text,
                "Failed subdivision for chunk size {chunk_size}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_prepare_open_close_map_unbalanced_open() {
        let html = "<b><i>Unbalanced tags";
        let tg = TokenGroup::from_string(html);
        let result = tg.prepare_open_close_map();
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    }

    #[test]
    fn test_prepare_open_close_map_unbalanced_close() {
        let html = "Unbalanced tags</i></b>";
        let tg = TokenGroup::from_string(html);
        let result = tg.prepare_open_close_map();
        assert!(
            matches!(result, Err(SplitError::UnbalancedToken(_))),
            "Expected UnbalancedToken error, got {:?}",
            result
        );
    }

    #[test]
    fn test_no_split_inside_normal_tags() -> TestResult {
        let html = "<div>Some text before.<no_split_tag>Do not split this part.</no_split_tag>Some text after.</div>";
        let tg = TokenGroup::from_string(html);
        let no_split = vec!["no_split_tag"];
        let max_chunk_size = 20;

        let result = tg.subdivide(max_chunk_size, &no_split);

        assert!(matches!(
            result,
            Err(SplitError::SubdividedExceedingTheLimit(_))
        ));
        let Err(SplitError::SubdividedExceedingTheLimit(tgs)) = result else {
            unreachable!()
        };

        assert!(tgs.iter().any(|tg| tg
            .to_string()
            .contains("<no_split_tag>Do not split this part.</no_split_tag>")));
        Ok(())
    }

    #[test]
    fn test_nested_no_split_tags_exceed_chunk_size() -> TestResult {
        let html = "<no_split_outer><no_split_inner>Nested content that is too long for the chunk size limit.</no_split_inner></no_split_outer>";
        let tg = TokenGroup::from_string(html);
        let no_split = vec!["no_split_outer", "no_split_inner"];
        let max_chunk_size = 10;

        let result = tg.subdivide(max_chunk_size, &no_split);

        assert!(
            matches!(result, Err(SplitError::SubdividedExceedingTheLimit(_))),
            "Expected SubdividedExceedingTheLimit error, got {:?}",
            result
        );

        Ok(())
    }

    #[test]
    fn test_sample1() -> TestResult {
        let html = include_str!("./test_data/sample1.html");
        let tgs = TokenGroup::from_string(html).subdivide(4000, &["a"])?;
        for tg in tgs {
            assert!(
                !(format!("{tg}").contains(r#"<pre><code class="language-rust"></code></pre>"#))
            )
        }
        Ok(())
    }
}
