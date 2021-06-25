# referendum

Referendum is a differential testing framework for Rust. It enables the observation of non-deterministic compiler behavior in the context of a project's unit tests.

## Prerequisites

Referedum uses rustup to manage Rust toolkits used in differential testing.

In order to install and configure rustup, visit [the rustup webpage](https://rustup.rs) for more information.

Each toolkit used by referendum must be pre-installed through rustup.

To check the status of installed toolkits:
```
rustup show
```

To install specific toolkits:
```
rustup install nightly-YYYY-MM-DD
```

Referendum is not meant to replace cargo test. Therefore, before runnning referendum, ensure that cargo test builds and runs successfully. Cargo test does not necessarily need to yield an entire suite of passing tests for referendum to effectively measure non-deterministic behavior. 

## Usage

To install:
```
cargo install cargo-referendum
```

To run:
```
cargo referendum -- <toolkit_name> <toolkit_name> <toolkit_name>
```

