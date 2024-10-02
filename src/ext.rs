pub(crate) trait SplitPosExt {
    fn split_with_respect_to_whitespace(&self, max_len: usize) -> Option<&str>;
    fn utf8_substring(&self, max_len: usize) -> Option<&str>;
}

impl SplitPosExt for str {
    fn split_with_respect_to_whitespace(&self, max_len: usize) -> Option<&str> {
        if max_len >= self.len() {
            return Some(self);
        }

        let trimmed = self
            .utf8_substring(max_len)?
            .trim_end_matches(|ch: char| !ch.is_whitespace());

        if trimmed.is_empty() {
            return Some(&self[..max_len]);
        }
        Some(trimmed)
    }

    fn utf8_substring(&self, max_len: usize) -> Option<&str> {
        if max_len == 0 || self.is_empty() {
            return Some("");
        }

        self.char_indices()
            .map(|(index, ch)| index + ch.len_utf8())
            .take_while(|&next_index| next_index <= max_len)
            .last()
            .map(|end_index| &self[..end_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_with_respect_to_whitespace() {
        let s = "hello world";
        let trimmed = s.split_with_respect_to_whitespace(7);
        assert_eq!(trimmed, Some("hello "));

        let trimmed = s.split_with_respect_to_whitespace(100500);
        assert_eq!(trimmed, Some("hello world"));

        let s = "long_word_with_no_whitespace";
        let trimmed = s.split_with_respect_to_whitespace(5);
        assert_eq!(trimmed, Some("long_"));

        let s = "italic bold strikethrough ";
        let trimmed = s.split_with_respect_to_whitespace(16);
        assert_eq!(trimmed, Some("italic bold "));
    }
}

#[cfg(test)]
mod tests_utf8_slice {
    use super::*;

    #[test]
    fn test_empty_string() {
        assert_eq!("".utf8_substring(5), Some(""));
    }

    #[test]
    fn test_max_len_zero() {
        assert_eq!("hello".utf8_substring(0), Some(""));
    }

    #[test]
    fn test_ascii_string() {
        assert_eq!("hello world".utf8_substring(5), Some("hello"));
    }

    #[test]
    fn test_partial_ascii_string() {
        assert_eq!("hello".utf8_substring(2), Some("he"));
    }

    #[test]
    fn test_non_ascii_string() {
        let s = "naïve";
        assert_eq!(s.utf8_substring(4), Some("naï"));
    }

    #[test]
    fn test_emoji_string() {
        let s = "👍👍👍";
        assert_eq!(s.utf8_substring(5), Some("👍"));
        assert_eq!(s.utf8_substring(8), Some("👍👍"));
        assert_eq!(s.utf8_substring(12), Some("👍👍👍"));
    }

    #[test]
    fn test_insufficient_max_len() {
        let s = "👍";
        assert_eq!(s.utf8_substring(3), None);
    }

    #[test]
    fn test_max_len_equals_char_len() {
        let s = "👍";
        assert_eq!(s.utf8_substring(4), Some("👍"));
    }

    #[test]
    fn test_german_umlaut() {
        let s = "über";
        assert_eq!(s.utf8_substring(1), None); // 'ü' is 2 bytes
        assert_eq!(s.utf8_substring(2), Some("ü"));
        assert_eq!(s.utf8_substring(3), Some("üb"));
    }

    #[test]
    fn test_full_string() {
        let s = "こんにちは";
        assert_eq!(s.utf8_substring(15), Some(s));
    }

    #[test]
    fn test_partial_multibyte_char() {
        let s = "こんにちは";
        assert_eq!(s.utf8_substring(2), None); // No character fits in 2 bytes
        assert_eq!(s.utf8_substring(3), Some("こ")); // First character is 3 bytes
        assert_eq!(s.utf8_substring(4), Some("こ")); // "こ" fits within 4 bytes
    }

    #[test]
    fn test_max_len_longer_than_string() {
        let s = "hello";
        assert_eq!(s.utf8_substring(10), Some("hello"));
    }
}
