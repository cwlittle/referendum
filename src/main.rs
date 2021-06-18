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

fn main() {
    let toolkits = [
        "nightly-2021-06-03-x86_64-apple-darwin",
        "nightly-x86_64-apple-darwin",
    ];

    for kit in toolkits.iter() {
        println!("{}", run_tests(kit));
    }
}
