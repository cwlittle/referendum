use fasthash::sea;
use regex::Regex;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fmt::Debug;
use std::process::{exit, Command};
use std::str;
use string_builder::Builder;

#[derive(thiserror::Error, Debug)]
pub enum ReferendumError {
    #[error("Test run failed to execute")]
    TestRunFailure(),
    #[error("Test run extraction failure")]
    TestResultExtractionFailure(),
    #[error("Tests not found failure")]
    TestNotFound(),
}

pub type Result<T> = std::result::Result<T, ReferendumError>;

fn run_tests(toolkit: &str) -> Result<String> {
    let output = Command::new("rustup")
        .arg("run")
        .arg(toolkit)
        .arg("cargo")
        .arg("test")
        .arg("--")
        .arg("--test-threads=1")
        .arg("--show-output")
        .output()
        .expect("Error running command");

    match &output.status.success() {
        false => {
            //want to add information about the error here too
            return Err(ReferendumError::TestRunFailure());
        }
        true => (),
    };

    match str::from_utf8(&output.stdout) {
        Ok(v) => Ok(v.to_string()),
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

fn get_test_result(test_name: &str, lines: &[String]) -> Result<bool> {
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
            return Ok(item.contains("ok"));
        }
    }
    Err(ReferendumError::TestResultExtractionFailure())
}

fn get_test_output(test_name: &str, lines: &[String]) -> Result<String> {
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
        return Ok("".to_string());
    }
    Ok(lines[start..=end].concat())
}

fn get_consensus_hash(tests: &Vec<Test>) -> Option<u64> {
    let mut map: HashMap<u64, u8> = HashMap::new();
    for test in tests {
        let count = map.entry(test.hash).or_insert(0);
        *count += 1;
    }

    let max_hash = map.iter().max_by(|a, b| a.1.cmp(&b.1)).unwrap();

    match max_hash.1 {
        1 => None,
        _ => Some(max_hash.0.clone()),
    }
}

pub fn vote(tests: Vec<Test>) -> Result<VoteResult> {
    let mut test_map: HashMap<String, Vec<Test>> = HashMap::new();
    for test in tests {
        let entry = test_map.entry(test.name.clone()).or_insert(Vec::new());
        entry.push(test);
    }

    let mut matches: Vec<Test> = Vec::new();
    let mut non_matches: Vec<Test> = Vec::new();
    let mut no_consensus: Vec<Test> = Vec::new();

    for (_name, test_list) in test_map.iter() {
        let consensus = match get_consensus_hash(test_list) {
            Some(hash) => hash,
            None => {
                for test in test_list {
                    no_consensus.push(test.clone());
                }
                continue;
            }
        };

        for test in test_list {
            match test.hash == consensus {
                true => matches.push(test.clone()),
                false => non_matches.push(test.clone()),
            }
        }
    }

    if matches.is_empty() && non_matches.is_empty() && no_consensus.is_empty() {
        return Err(ReferendumError::TestNotFound());
    }

    Ok(VoteResult {
        matches,
        non_matches,
        no_consensus,
    })
}

pub fn get_tests(toolkits: Vec<&str>) -> Result<Vec<Test>> {
    let mut tests: Vec<Test> = Vec::new();
    //should I add something here to check that if a toolkit is installed in rustup
    for kit in toolkits {
        let run = &run_tests(kit)?;
        let lines = parse_test_output(run);
        let unique_test_names = get_test_names(&lines);
        for test in unique_test_names.iter() {
            let test_output = get_test_output(test, &lines)?;
            let test_result = get_test_result(test, &lines)?;
            let output_obj = Test {
                name: test.clone(),
                toolkit: kit.to_string(),
                result: test_result,
                output: test_output.clone(),
                hash: sea::hash64((test_result.to_string() + &test_output).as_bytes()),
            };
            tests.push(output_obj);
        }
    }
    Ok(tests)
}

fn generate_test_result_output(name: &str, result: bool, toolkit: Option<&str>) -> String {
    let mut builder = Builder::default();
    builder.append("test ");
    builder.append(name.to_string());
    match toolkit {
        Some(kit) => {
            builder.append(" @ ");
            builder.append(kit);
        }
        None => builder.append(" "),
    }
    builder.append(" ... ");
    if result {
        builder.append("ok");
    } else {
        builder.append("FAILED");
    }
    builder.string().unwrap()
}

fn generate_test_output_output(name: &str, output: &str, toolkit: Option<&str>) -> String {
    let mut builder = Builder::default();
    builder.append("\n\t---- test ");
    builder.append(name);
    match toolkit {
        Some(kit) => {
            builder.append(" @ ");
            builder.append(kit);
        }
        None => (),
    }
    builder.append(" stdout ----\n");
    builder.append("\t");
    builder.append(output);
    builder.append("\n");

    builder.string().unwrap()
}

pub fn generate_consensus_map(consensus_votes: &Vec<Test>) -> HashMap<String, Consensus> {
    let mut consensus_map: HashMap<String, Consensus> = HashMap::new();
    for matched_vote in consensus_votes.iter() {
        if !consensus_map.contains_key(&matched_vote.name.to_string()) {
            let consensus = Consensus {
                name: matched_vote.name.clone(),
                result: matched_vote.result,
                output: matched_vote.output.clone(),
            };
            consensus_map.insert(consensus.name.to_string(), consensus);
        }
    }
    consensus_map
}

pub fn get_consensus_results(consensus_map: &HashMap<String, Consensus>) -> String {
    let mut builder = Builder::default();
    builder.append("Consensus Test Results...\n");
    for (name, vote) in consensus_map.iter() {
        builder.append(generate_test_result_output(
            &name,
            vote.result,
            Some("consensus"),
        ));
        if !vote.output.is_empty() {
            builder.append(generate_test_output_output(
                &name,
                &vote.output,
                Some("consensus"),
            ));
        }
        builder.append("\n");
    }

    builder.string().unwrap()
}

pub fn get_dissenting_results(
    dissenting_votes: Vec<Test>,
    consensus_map: &HashMap<String, Consensus>,
) -> String {
    let mut builder = Builder::default();
    builder.append("Dissenting Test Results...\n");
    for dissenting_vote in dissenting_votes.iter() {
        let consensus = consensus_map
            .get(&dissenting_vote.name)
            .expect("Non-matched test vote does not have corresponding consensus");

        builder.append(generate_test_result_output(
            &consensus.name,
            consensus.result,
            Some("consensus"),
        ));
        builder.append("\n");

        builder.append(generate_test_result_output(
            &dissenting_vote.name,
            dissenting_vote.result,
            Some(&dissenting_vote.toolkit),
        ));

        builder.append(generate_test_output_output(
            &consensus.name,
            &consensus.output,
            Some("consensus"),
        ));
        builder.append(generate_test_output_output(
            &dissenting_vote.name,
            &dissenting_vote.output,
            Some(&dissenting_vote.toolkit),
        ));

        builder.append("\n");
    }
    builder.string().unwrap()
}

pub fn get_no_consensus_results(no_consensus_votes: Vec<Test>) -> String {
    let mut builder = Builder::default();
    builder.append("No Consensus Results...\n");
    for vote in no_consensus_votes.iter() {
        builder.append(generate_test_result_output(
            &vote.name,
            vote.result,
            Some(&vote.toolkit),
        ));
        builder.append(generate_test_output_output(
            &vote.name,
            &vote.output,
            Some(&vote.toolkit),
        ));
        builder.append("\n");
    }
    builder.string().unwrap()
}

#[derive(Debug, Clone)]
pub struct Test {
    pub name: String,
    pub toolkit: String,
    pub result: bool,
    pub output: String,
    pub hash: u64,
}

#[derive(Debug)]
pub struct Consensus {
    pub name: String,
    pub result: bool,
    pub output: String,
}

#[derive(Debug)]
pub struct VoteResult {
    pub matches: Vec<Test>,
    pub non_matches: Vec<Test>,
    pub no_consensus: Vec<Test>,
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
        assert_eq!(get_test_output(&test_name, &lines).unwrap(), "Hello World");
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
        assert_eq!(get_test_output(&test_name, &lines).unwrap(), "Hello World");
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
        assert_eq!(get_test_output(&test_name, &lines).unwrap(), "Hello World");
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
        assert_eq!(get_test_output(&test_name, &lines).unwrap(), "Hello World");
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
        assert_eq!(
            get_test_output(&test_name, &lines).unwrap(),
            "Hello, Earthlings!"
        );
    }

    #[test]
    fn get_consensus() {
        let test_1 = Test {
            name: "test_1".to_string(),
            toolkit: "nightly".to_string(),
            result: true,
            output: "test output".to_string(),
            hash: 42,
        };

        let test_2 = Test {
            name: "test_2".to_string(),
            toolkit: "nightly".to_string(),
            result: false,
            output: "test output".to_string(),
            hash: 42,
        };

        let test_3 = Test {
            name: "test_3".to_string(),
            toolkit: "nightly".to_string(),
            result: false,
            output: "test output".to_string(),
            hash: 12,
        };

        let tests: Vec<Test> = vec![test_1, test_2, test_3];
        let consensus = get_consensus_hash(&tests);
        assert_eq!(consensus, Some(42));
    }

    #[test]
    fn get_test_result_normal() {
        let test_name = "testing::test_name";
        let lines = ["test testing::test_name ... ok".to_string()];

        assert!(get_test_result(&test_name, &lines).unwrap());
    }

    #[test]
    fn get_test_result_test_panics() {
        let test_name = "testing::test_name";
        let lines = ["test testing::test_name - should panic ... ok".to_string()];

        assert!(get_test_result(&test_name, &lines).unwrap());
    }

    #[test]
    fn vote_all_consensus() {
        let test_1 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_1".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let test_2 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_2".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let test_3 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_3".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let tests = vec![test_1, test_2, test_3];
        let votes = vote(tests).unwrap();
        assert_eq!(votes.matches.len(), 3);
        assert_eq!(votes.non_matches.len(), 0);
        assert_eq!(votes.no_consensus.len(), 0);
    }

    #[test]
    fn vote_all_unmatched() {
        let test_1 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_1".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let test_2 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_2".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let test_3 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_3".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 12,
        };
        let tests = vec![test_1, test_2, test_3];
        let votes = vote(tests).unwrap();
        assert_eq!(votes.matches.len(), 2);
        assert_eq!(votes.non_matches.len(), 1);
        assert_eq!(votes.no_consensus.len(), 0);
    }

    #[test]
    fn vote_no_consensus() {
        let test_1 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_1".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let test_2 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_2".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 44,
        };
        let test_3 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_3".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 12,
        };
        let tests = vec![test_1, test_2, test_3];
        let votes = vote(tests).unwrap();
        assert_eq!(votes.matches.len(), 0);
        assert_eq!(votes.non_matches.len(), 0);
        assert_eq!(votes.no_consensus.len(), 3);
    }

    #[test]
    fn test_output_generation() {
        let output =
            generate_test_output_output(&"test_name", &"this is the output", Some("tester"));
        let expected = "\n\t---- test test_name @ tester stdout ----\n\tthis is the output\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_output_generation_no_output() {
        let output = generate_test_output_output(&"test_name", &"", Some("tester"));
        let expected = "\n\t---- test test_name @ tester stdout ----\n\t\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_pass_result_generation() {
        let output = generate_test_result_output(&"test_name", true, Some("tester"));
        let expected = "test test_name @ tester ... ok";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_failure_result_generation() {
        let output = generate_test_result_output(&"test_name", false, Some("tester"));
        let expected = "test test_name @ tester ... FAILED";
        assert_eq!(output, expected);
    }

    #[test]
    fn consensus_map_normal_generation() {
        let test_1 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_1".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let test_2 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_2".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 44,
        };
        let test_3 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_3".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let tests = vec![test_1, test_2, test_3];

        assert_eq!(format!("{:?}", generate_consensus_map(&tests)),
            "{\"test_name\": Consensus { name: \"test_name\", result: true, output: \"this is the output\" }}");
    }

    #[test]
    fn consensus_result_normal_generation() {
        let test_1 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_1".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let test_2 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_2".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 44,
        };
        let test_3 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_3".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let tests = vec![test_1, test_2, test_3];
        let map = generate_consensus_map(&tests);
        assert_eq!(get_consensus_results(&map),
            "Consensus Test Results...\ntest test_name @ consensus ... ok\n\t---- test test_name @ consensus stdout ----\n\tthis is the output\n\n");
    }

    #[test]
    fn dissenting_result_normal_generation() {
        let test_1 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_1".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let test_2 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_2".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 44,
        };
        let test_3 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_3".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 42,
        };
        let test_4 = test_2.clone();
        let tests = vec![test_1, test_2, test_3];
        let map = generate_consensus_map(&tests);
        assert_eq!(get_dissenting_results(vec![test_4], &map),
            "Dissenting Test Results...\ntest test_name @ consensus ... ok\ntest test_name @ nightly_2 ... ok\n\t---- test test_name @ consensus stdout ----\n\tthis is the output\n\n\t---- test test_name @ nightly_2 stdout ----\n\tthis is the output\n\n");
    }

    #[test]
    fn no_consensus_result_normal_generation() {
        let test_1 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_1".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 12,
        };
        let test_2 = Test {
            name: "test_name".to_string(),
            toolkit: "nightly_2".to_string(),
            result: true,
            output: "this is the output".to_string(),
            hash: 44,
        };
        let tests = vec![test_1, test_2];
        assert_eq!(get_no_consensus_results(tests),
            "No Consensus Results...\ntest test_name @ nightly_1 ... ok\n\t---- test test_name @ nightly_1 stdout ----\n\tthis is the output\n\ntest test_name @ nightly_2 ... ok\n\t---- test test_name @ nightly_2 stdout ----\n\tthis is the output\n\n");
    }
}
