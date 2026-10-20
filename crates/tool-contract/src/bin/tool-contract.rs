//! `tool-contract validate <dir>`: finds every `contract.yaml` under `<dir>`, validates each
//! against the `tool-contract/1` profile, and checks each sibling `scenarios/` directory covers
//! the profile. Prints `ok <path>` per accepted document or one line per refusal; exits 1 on any.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tool_contract::conformance::{Suite, coverage};
use tool_contract::{Contract, validate};

#[derive(Parser)]
#[command(
    name = "tool-contract",
    about = "Checks tool-contract/1 documents",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate every contract.yaml under DIR and the coverage of each sibling scenarios/.
    Validate {
        /// The directory to walk.
        dir: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Validate { dir } => validate_dir(&dir),
    }
}

fn validate_dir(dir: &Path) -> ExitCode {
    let mut documents = Vec::new();
    if let Err(err) = find_contracts(dir, &mut documents) {
        eprintln!("{}: {err}", dir.display());
        return ExitCode::from(1);
    }
    documents.sort();
    if documents.is_empty() {
        println!("no contract documents under {}", dir.display());
        return ExitCode::SUCCESS;
    }
    let mut refused = false;
    for path in documents {
        match check_document(&path) {
            Ok(()) => println!("ok {}", path.display()),
            Err(lines) => {
                refused = true;
                for line in lines {
                    println!("{}: {line}", path.display());
                }
            }
        }
    }
    if refused {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn find_contracts(dir: &Path, found: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            find_contracts(&path, found)?;
        } else if path.file_name().is_some_and(|name| name == "contract.yaml") {
            found.push(path);
        }
    }
    Ok(())
}

fn check_document(path: &Path) -> Result<(), Vec<String>> {
    let text = fs::read_to_string(path).map_err(|err| vec![format!("cannot read: {err}")])?;
    let contract = Contract::from_yaml(&text).map_err(|err| vec![err.to_string()])?;
    let mut lines: Vec<String> = match validate(&contract) {
        Ok(()) => Vec::new(),
        Err(violations) => violations.iter().map(ToString::to_string).collect(),
    };
    let scenarios = path.parent().map(|dir| dir.join("scenarios"));
    if let Some(scenarios) = scenarios.filter(|dir| dir.is_dir()) {
        match Suite::read_dir(&scenarios) {
            Ok(suite) => {
                if let Err(refused) = coverage(&suite, &contract) {
                    lines.extend(refused.into_iter().map(|line| format!("scenarios: {line}")));
                }
            }
            Err(err) => lines.push(format!("scenarios: {err}")),
        }
    }
    if lines.is_empty() { Ok(()) } else { Err(lines) }
}
