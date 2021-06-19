use regex::Regex;
use std::collections::BTreeSet;
use std::process::Command;
use std::str;
use string_builder::Builder;

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

fn get_test_names(lines: &Vec<String>) -> BTreeSet<String> {
    let re = Regex::new("[a-zA-z_0-9]+::[a-zA-Z_0-9]+").unwrap();
    let lines_string = lines.join(" ");
    re.find_iter(&lines_string)
        .filter_map(|digits| digits.as_str().parse().ok())
        .collect()
}

fn get_test_result(test_name: &String, lines: &Vec<String>) -> bool {
    let re = Regex::new("[a-zA-z_0-9]+::[a-zA-Z_0-9]+ ... (ok|FAILED)").unwrap();
    let lines_string = lines.join(" ");
    let result_lines: Vec<String> = re
        .find_iter(&lines_string)
        .filter_map(|digits| digits.as_str().parse().ok())
        .collect();

    for item in result_lines {
        if item.contains(test_name) {
            return item.contains("ok");
        }
    }

    panic!("You shouldn't be here");
}

fn get_test_output(test_name: &String, lines: &Vec<String>) -> String {
    let mut start: usize = 0;
    let mut end: usize = lines.len() - 1;

    let mut builder = Builder::default();
    builder.append("---- ");
    builder.append(test_name.clone());
    builder.append(" stdout ----");
    let start_string = builder.string().unwrap();
    let start_re = Regex::new(&start_string).unwrap();
    let next_test_re = Regex::new(r"---- [a-zA-Z_0-9]+::[a-zA-Z_0-9]+ stdout ----").unwrap();
    let failure_re = Regex::new(r"failures:").unwrap();

    for (i, line) in lines.iter().enumerate() {
        if start_re.is_match(line) {
            start = i + 1;
        } else if next_test_re.is_match(line) || failure_re.is_match(line) {
            end = i - 1;
            break;
        }
    }
    lines[start..=end].concat()
}

fn main() {
    let toolkits = [
        "nightly-2021-06-03-x86_64-apple-darwin",
        "nightly-x86_64-apple-darwin",
    ];

    for kit in toolkits {
        let lines = parse_test_output(&run_tests(kit));
        let unique_test_names = get_test_names(&lines);
        for test in unique_test_names.iter() {
            let test_result = get_test_result(&test, &lines);
            let test_output = get_test_output(&test, &lines);
        }
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

    #[test]
    fn extract_test_name() {
        let input = vec![String::from("test tests::test_1 ... ok")];
        let mut expected = BTreeSet::new();
        expected.insert(String::from("tests::test_1"));
        assert_eq!(get_test_names(&input), expected);
    }

    #[test]
    fn extract_test_output_break_end() {
        let test_name = String::from("tests::test_1");
        let lines = vec![
            String::from("what"),
            String::from("---- tests::test_1 stdout ----"),
            String::from("Hello "),
            String::from("World"),
        ];
        assert_eq!(get_test_output(&test_name, &lines), "Hello World");
    }

    #[test]
    fn extract_test_output_break_next_test() {
        let test_name = String::from("tests::test_1");
        let lines = vec![
            String::from("what"),
            String::from("---- tests::test_1 stdout ----"),
            String::from("Hello "),
            String::from("World"),
            String::from("---- tests::test_2 stdout ----"),
        ];
        assert_eq!(get_test_output(&test_name, &lines), "Hello World");
    }

    #[test]
    fn extract_test_output_break_failures() {
        let test_name = String::from("tests::test_1");
        let lines = vec![
            String::from("what"),
            String::from("---- tests::test_1 stdout ----"),
            String::from("Hello "),
            String::from("World"),
            String::from("failures:"),
        ];
        assert_eq!(get_test_output(&test_name, &lines), "Hello World");
    }
}
