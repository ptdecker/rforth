use rforth::tokenizer::{WordVec, parse_words};

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
