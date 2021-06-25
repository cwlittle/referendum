use clap::{crate_version, App, Arg, SubCommand};
use referendum::*;
use std::process::exit;

fn main() {
    let args = App::new("cargo-referendum")
        .author("Charlie Little, <cwlittle@utexas.edu>")
        .about("Differential testing tool for unit tests")
        .version(concat!("version: ", crate_version!()))
        .bin_name("cargo")
        .subcommand(
            SubCommand::with_name("referendum")
                .about("Differential testing tool for unit tests")
                .version(concat!("version: ", crate_version!()))
                .arg(Arg::with_name("toolkits").required(true).min_values(1)),
        )
        .get_matches();

    let mut toolkits: Vec<_> = Vec::new();
    if let Some(args) = args.subcommand_matches("referendum") {
        toolkits = args.values_of("toolkits").unwrap().collect();
    } else {
        exit(1);
    }

    let old_toolkits = vec![
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
