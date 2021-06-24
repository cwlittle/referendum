use referendum::*;
use std::process::exit;

fn main() {
    let toolkits = vec![
        "nightly-2021-06-03-x86_64-apple-darwin",
        "nightly-x86_64-apple-darwin",
    ];

    //check that all toolkits are installed before running this
    let tests = match get_tests(toolkits) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            exit(1);
        }
    };
    let votes = match vote(tests) {
        Ok(v) => v,
        Err(_e) => {
            println!("No tests found");
            std::process::exit(1);
        }
    };

    let consensus_map = generate_consensus_map(&votes.matches);

    if !votes.matches.is_empty() {
        println!("{}", get_consensus_results(&consensus_map));
    } else {
        println!("No consensus determined among test ouputs ...\n");
    }

    if !votes.non_matches.is_empty() {
        println!(
            "{}",
            get_dissenting_results(votes.non_matches, &consensus_map)
        );
    } else {
        println!("No dissenting test outputs found ...\n");
    }

    if !votes.no_consensus.is_empty() {
        println!("{}", get_no_consensus_results(votes.no_consensus));
    }
}
