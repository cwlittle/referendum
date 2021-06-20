use fasthash::sea;
use regex::Regex;
use std::collections::BTreeSet;
use std::collections::HashMap;
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
        .arg("--test-threads=1")
        .output()
        .expect("Error running command");

    match str::from_utf8(&output.stdout) {
        Ok(v) => v.to_string(),
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    }
}

fn parse_test_output(output: &str) -> Vec<String> {
    let lines: Vec<String> = output.split('\n').map(|x| x.to_string()).collect();

    lines
}

fn get_test_names(lines: &[String]) -> BTreeSet<String> {
    let re = Regex::new("test [a-zA-z_0-9]+::[a-zA-Z_0-9]+").unwrap();
    let lines_string = lines.join(" ");
    let names: Vec<String> = re
        .find_iter(&lines_string)
        .filter_map(|digits| digits.as_str().parse().ok())
        .collect();
    names
        .iter()
        .map(|name| name.replacen("test ", "", 1))
        .collect()
}

fn get_test_result(test_name: &str, lines: &[String]) -> bool {
    let re = Regex::new("[a-zA-z_0-9]+::[a-zA-Z_0-9]+ ... (ok|FAILED)").unwrap();
    let re_should_panic =
        Regex::new("[a-zA-z_0-9]+::[a-zA-Z_0-9]+ - should panic ... (ok|FAILED)").unwrap();
    let lines_string = lines.join(" ");

    let mut result_lines: Vec<String> = re
        .find_iter(&lines_string)
        .filter_map(|digits| digits.as_str().parse().ok())
        .collect();

    let panic_result_lines: Vec<String> = re_should_panic
        .find_iter(&lines_string)
        .filter_map(|digits| digits.as_str().parse().ok())
        .collect();

    result_lines.extend(panic_result_lines);

    for item in result_lines.iter() {
        if item.contains(test_name) {
            return item.contains("ok");
        }
    }

    panic!("Error getting test results");
}

fn get_test_output(test_name: &str, lines: &[String]) -> String {
    //TODO: Clean this, it's repulsive
    let mut start: usize = 0;
    let mut end: usize = lines.len() - 1;

    let mut builder = Builder::default();
    builder.append("---- ");
    builder.append(test_name.to_string());
    builder.append(" stdout ----");
    let start_string = builder.string().unwrap();
    let start_re = Regex::new(&start_string).unwrap();
    let next_test_re = Regex::new(r"---- [a-zA-Z_0-9]+::[a-zA-Z_0-9]+ stdout ----").unwrap();
    let failure_re = Regex::new(r"failures:").unwrap();
    let success_re = Regex::new(r"successes:").unwrap();

    let mut start_found = false;
    for (i, line) in lines.iter().enumerate() {
        if start_re.is_match(line) {
            start_found = true;
            start = i + 1;
        } else if next_test_re.is_match(line)
            || failure_re.is_match(line)
            || success_re.is_match(line) && start_found && i - 1 > start
        {
            end = i - 1;
            break;
        }
    }
    if !start_found {
        return "".to_string();
    }
    lines[start..=end].concat()
}

fn get_consensus_hash(tests: &Vec<Test>) -> (u64, String) {
    let mut map: HashMap<u64, u8> = HashMap::new();
    for test in tests {
        let count = map.entry(test.hash).or_insert(0);
        *count += 1;
    }

    let max_hash = map
        .iter()
        .max_by(|a, b| a.1.cmp(&b.1))
        .map(|(k, _v)| k)
        .unwrap();

    let mut max_hash_output = String::new();
    for test in tests {
        if test.hash == *max_hash {
            max_hash_output = test.output.clone();
        }
    }
    //need another loop to account for when there is no consensus
    (max_hash.clone(), max_hash_output)
}

#[derive(Debug)]
struct Test {
    name: String,
    toolkit: String,
    result: bool,
    output: String,
    hash: u64,
}

fn main() {
    let toolkits = [
        "nightly-2021-06-03-x86_64-apple-darwin",
        "nightly-x86_64-apple-darwin",
    ];

    let mut tests: Vec<Test> = Vec::new();
    for kit in toolkits {
        let lines = parse_test_output(&run_tests(kit));
        let unique_test_names = get_test_names(&lines);
        for test in unique_test_names.iter() {
            let test_output = get_test_output(test, &lines);
            let output_obj = Test {
                name: test.clone(),
                toolkit: kit.to_string(),
                result: get_test_result(test, &lines),
                output: test_output.clone(),
                hash: sea::hash64(test_output.as_bytes()),
            };
            tests.push(output_obj);
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

    #[test]
    fn extract_test_output_break_successes() {
        let test_name = String::from("tests::test_1");
        let lines = vec![
            String::from("what"),
            String::from("---- tests::test_1 stdout ----"),
            String::from("Hello "),
            String::from("World"),
            String::from(""),
            String::from("successes:"),
        ];
        assert_eq!(get_test_output(&test_name, &lines), "Hello World");
    }

    #[test]
    fn extract_test_output_real() {
        let test_name = String::from("tests::test_1");
        let lines = [
            "", 
            "running 2 tests", 
            "test tests::test_1 ... ok", 
            "test tests::test_2 ... FAILED", 
            "", 
            "successes:", 
            "", 
            "---- tests::test_1 stdout ----", 
            "Hello, Earthlings!", 
            "", 
            "", 
            "successes:", 
            "    tests::test_1", 
            "", 
            "failures:", 
            "", 
            "---- tests::test_2 stdout ----", 
            "thread 'main' panicked at 'assertion failed: `(left == right)`", 
            "  left: `1`,", " right: `2`', src/main.rs:14:9", 
            "note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace", 
            "", 
            "", 
            "failures:", 
            "    tests::test_2", 
            "", 
            "test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s", 
            "", 
            ""
        ];
        let lines: Vec<String> = lines.iter().map(|x| x.to_string()).collect();
        assert_eq!(get_test_output(&test_name, &lines), "Hello, Earthlings!");
    }

    #[test]
    fn get_consensus() {
        let test_1 = Test {
            name: "test_1".to_string(),
            result: true,
            output: "test output".to_string(),
            hash: 42,
        };

        let test_2 = Test {
            name: "test_2".to_string(),
            result: false,
            output: "test output".to_string(),
            hash: 42,
        };

        let test_3 = Test {
            name: "test_3".to_string(),
            result: false,
            output: "test output".to_string(),
            hash: 12,
        };

        let tests: Vec<Test> = vec![test_1, test_2, test_3];
        let consensus = get_consensus_hash(&tests);
        assert_eq!(consensus, (42, "test output".to_string()));
    }
}
