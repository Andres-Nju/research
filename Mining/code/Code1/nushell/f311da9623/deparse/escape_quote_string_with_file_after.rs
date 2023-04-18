pub fn escape_quote_string_with_file(input: &str, file: &str) -> String {
    // use when you want to cross-compare to a file to ensure flags are checked properly
    let file = File::open(file);
    match file {
        Ok(f) => {
            let lines = BufReader::new(f).lines();
            for line in lines {
                let mut flag_start = false;
                let mut word = String::new();
                let line_or = line.unwrap_or_else(|_| String::from(" "));
                if line_or.contains('-') {
                    for n in line_or.chars() {
                        if n == '-' {
                            flag_start = true;
                        }
                        if n == ' ' || n == ':' || n == ')' {
                            flag_start = false;
                        }
                        if flag_start {
                            word.push(n);
                        }
                    }
                }
                if word.contains(input) {
                    return input.to_string();
                }
            }
            let mut final_word = String::new();
            final_word.push('"');
            final_word.push_str(input);
            final_word.push('"');
            final_word
        }
        _ => escape_quote_string_when_flags_are_unclear(input),
    }
}
