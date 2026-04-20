use rforth::{io::ForthIo, run_forth_steps};

struct ScriptedIo<'a> {
    input: &'a [u8],
    input_pos: usize,
    output: Vec<u8>,
}

impl<'a> ScriptedIo<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            input_pos: 0,
            output: Vec::new(),
        }
    }

    fn output(&self) -> &[u8] {
        &self.output
    }
}

impl ForthIo for ScriptedIo<'_> {
    fn emit(&mut self, c: u8) {
        self.output.push(c);
    }

    fn key(&mut self) -> u8 {
        let c = self.input[self.input_pos];
        self.input_pos += 1;
        c
    }
}

#[test]
fn emits_startup_banner_without_reading_input() {
    let mut io = ScriptedIo::new(b"");

    run_forth_steps(&mut io, 0);

    assert_eq!(io.output(), b"OK\n");
}

#[test]
fn echoes_newline_and_outputs_tokenized_words() {
    let mut io = ScriptedIo::new(b"one two\n");

    run_forth_steps(&mut io, b"one two\n".len());

    assert_eq!(io.output(), b"OK\none two\n[one, two]\n");
}

#[test]
fn carriage_return_echoes_newline_before_words() {
    let mut io = ScriptedIo::new(b"one two\r");

    run_forth_steps(&mut io, b"one two\r".len());

    assert_eq!(io.output(), b"OK\none two\r\n[one, two]\n");
}

#[test]
fn resets_line_after_each_completed_input_line() {
    let mut io = ScriptedIo::new(b"one two\nthree\n");

    run_forth_steps(&mut io, b"one two\nthree\n".len());

    assert_eq!(io.output(), b"OK\none two\n[one, two]\nthree\n[three]\n");
}

#[test]
fn ignores_input_bytes_after_line_buffer_is_full() {
    let input = [b'a'; 130];
    let mut io = ScriptedIo::new(&input);

    run_forth_steps(&mut io, input.len());

    assert_eq!(io.output(), &[b"OK\n".as_slice(), input.as_slice()].concat());
}
