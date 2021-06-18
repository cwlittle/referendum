use std::process::Command;

use std::str;

fn run_tests(toolkit: &str) -> String {
    let output = Command::new("rustup")
        .arg("run")
        .arg(toolkit)
        .arg("cargo")
        .arg("test")
        .arg("--")
        .arg("--show-output")
        .output()
        .expect("Error running command");

    match str::from_utf8(&output.stdout) {
        Ok(v) => v.to_string(),
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    }
}

fn parse_test_output(output: &str) -> Vec<String> {
    let lines: Vec<String> = output.split("\n").map(|x| x.to_string()).collect();

    lines
}

fn main() {
    let toolkits = [
        "nightly-2021-06-03-x86_64-apple-darwin",
        "nightly-x86_64-apple-darwin",
    ];

    for kit in toolkits {
        let lines = parse_test_output(&run_tests(kit));
        println!("toolkit: {}", kit);
        println!("{:?}", lines);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_test() {
        let input = "test tests::test_1 ... ok";
        let expected = vec!["test tests::test_1 ... ok"];
        assert_eq!(parse_test_output(input), expected);
    }

    #[test]
    fn parse_double_test() {
        let input = "Hello\nWorld";
        let expected = vec!["Hello", "World"];
        assert_eq!(parse_test_output(input), expected);
    }
}
