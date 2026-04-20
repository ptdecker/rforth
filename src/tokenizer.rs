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

impl<'a, const N: usize> Default for WordVec<'a, N> {
    fn default() -> Self {
        Self::new()
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
