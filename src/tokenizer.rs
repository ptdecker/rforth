//! Token parsing for rforth input lines
//!
//! The tokenizer is intentionally allocation-free. It borrows word slices directly from the input
//! line and stores them in a fixed-capacity vector-like buffer supplied by this module.

/// Fixed-capacity collection of tokenized words.
pub struct WordVec<'a, const N: usize> {
    words: [&'a [u8]; N],
    len: usize,
}

impl<'a, const N: usize> WordVec<'a, N> {
    /// Construct an empty word vector.
    pub fn new() -> Self {
        Self {
            words: [&[]; N],
            len: 0,
        }
    }

    /// Add a word to the vector.
    ///
    /// Returns `false` when the vector is already full.
    pub fn push(&mut self, word: &'a [u8]) -> bool {
        if self.len == N {
            return false;
        }
        self.words[self.len] = word;
        self.len += 1;
        true
    }

    /// Return the collected words as a slice.
    pub fn as_slice(&self) -> &[&'a [u8]] {
        &self.words[..self.len]
    }
}

/// Parse ASCII whitespace-separated words out of an input line.
pub fn parse_words<const N: usize>(input: &[u8]) -> WordVec<'_, N> {
    let mut words = WordVec::new();
    let mut pos = 0;

    while pos < input.len() {
        while pos < input.len() && input[pos].is_ascii_whitespace() {
            pos += 1;
        }

        let start = pos;
        while pos < input.len() && !input[pos].is_ascii_whitespace() {
            pos += 1;
        }

        if start != pos && !words.push(&input[start..pos]) {
            break;
        }
    }

    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_words_split_by_spaces() {
        let words = parse_words::<8>(b"one two three");

        assert_eq!(words.as_slice(), &[&b"one"[..], &b"two"[..], &b"three"[..]]);
    }

    #[test]
    fn ignores_leading_trailing_and_repeated_whitespace() {
        let words = parse_words::<8>(b"  one\t two\n\rthree  ");

        assert_eq!(words.as_slice(), &[&b"one"[..], &b"two"[..], &b"three"[..]]);
    }

    #[test]
    fn stops_when_word_vector_is_full() {
        let words = parse_words::<2>(b"one two three");

        assert_eq!(words.as_slice(), &[&b"one"[..], &b"two"[..]]);
    }

    #[test]
    fn empty_input_produces_no_words() {
        let words = parse_words::<8>(b"");

        assert!(words.as_slice().is_empty());
    }

    #[test]
    fn all_whitespace_input_produces_no_words() {
        let words = parse_words::<8>(b" \t\n\r ");

        assert!(words.as_slice().is_empty());
    }

    #[test]
    fn zero_capacity_produces_no_words() {
        let words = parse_words::<0>(b"one two");

        assert!(words.as_slice().is_empty());
    }

    #[test]
    fn keeps_non_whitespace_bytes_inside_words() {
        let words = parse_words::<8>(b": square dup * ;");

        assert_eq!(
            words.as_slice(),
            &[&b":"[..], &b"square"[..], &b"dup"[..], &b"*"[..], &b";"[..]]
        );
    }

    #[test]
    fn word_vec_reports_full_push_failure() {
        let mut words = WordVec::<2>::new();

        assert!(words.push(b"one"));
        assert!(words.push(b"two"));
        assert!(!words.push(b"three"));
        assert_eq!(words.as_slice(), &[&b"one"[..], &b"two"[..]]);
    }
}
