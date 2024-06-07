#[test]
fn errors() {
    generate_snippets().unwrap_or_else(|err| panic!("{err}"));
    ::trybuild::TestCases::new().compile_fail("tests/errors/*.rs");
}

fn generate_snippets() -> Result<(), Box<dyn ::std::error::Error>> {
    use ::std::{fs, io::Write as _};

    fs::remove_dir_all("tests/errors")?;
    fs::create_dir_all("tests/errors")?;

    let file = fs::read_to_string("examples/errors.md")?;
    let mut lines = file.lines().map(|s| (s.trim(), s)).peekable();
    let mut parser_state = State::MarkdownText;
    // where
    enum State {
        MarkdownText,
        RustCode(fs::File),
        ErrorCode(fs::File),
        OtherCode,
    }
    let mut i = 1;
    while let Some((line, raw_line)) = lines.next() {
        if line.starts_with("```") {
            parser_state = match parser_state {
                State::MarkdownText => match line {
                    "```rs" | "```rust" => {
                        let filename = &format!("tests/errors/snippet_{i:02}.rs");
                        i += 1;
                        let mut file = fs::File::create(filename)?;
                        writeln!(file, "fn main() {{}}")?;
                        State::RustCode(file)
                    }
                    "```rust error" => {
                        let filename = &format!("tests/errors/snippet_{:02}.stderr", i - 1);
                        State::ErrorCode(fs::File::create(filename)?)
                    }
                    _ => State::OtherCode,
                },
                State::RustCode { .. } | State::ErrorCode { .. } | State::OtherCode { .. } => {
                    assert_eq!(line, "```");
                    State::MarkdownText
                }
            };
        } else {
            if let State::RustCode(file) | State::ErrorCode(file) = &parser_state {
                writeln!({ file }, "{raw_line}")?;
            }
        }
    }
    Ok(())
}
