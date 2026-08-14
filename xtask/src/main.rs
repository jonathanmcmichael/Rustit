use std::{env, error::Error, io, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VerificationStep {
    name: &'static str,
    args: &'static [&'static str],
}

const VERIFY_STEPS: &[VerificationStep] = &[
    VerificationStep {
        name: "formatting",
        args: &["fmt", "--all", "--", "--check"],
    },
    VerificationStep {
        name: "workspace check",
        args: &["check", "--workspace", "--all-targets", "--locked"],
    },
    VerificationStep {
        name: "strict linting",
        args: &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    },
    VerificationStep {
        name: "workspace tests and truth labs",
        args: &["test", "--workspace", "--all-targets", "--locked"],
    },
];

const LAB_STEP: VerificationStep = VerificationStep {
    name: "truth labs",
    args: &[
        "test",
        "--package",
        "rustit-fixtures",
        "--test",
        "truth_labs",
        "--locked",
    ],
};

fn main() -> Result<(), Box<dyn Error>> {
    match env::args().nth(1).as_deref() {
        Some("verify") => run_steps(VERIFY_STEPS),
        Some("labs") => run_steps(&[LAB_STEP]),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_help();
            Ok(())
        }
        Some(command) => Err(io::Error::other(format!(
            "unknown xtask command `{command}`; run `cargo xtask help`"
        ))
        .into()),
    }
}

fn run_steps(steps: &[VerificationStep]) -> Result<(), Box<dyn Error>> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    for step in steps {
        println!("\n==> Rustit {}", step.name);
        let status = Command::new(&cargo).args(step.args).status()?;
        if !status.success() {
            return Err(io::Error::other(format!("{} failed with {status}", step.name)).into());
        }
    }
    println!("\nRustit verification passed.");
    Ok(())
}

fn print_help() {
    println!("Rustit repository tasks\n");
    println!("  cargo xtask verify  Run the complete local contribution gate");
    println!("  cargo xtask labs    Run only the synthetic domain truth labs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_gate_includes_format_check_lint_and_tests() {
        assert_eq!(VERIFY_STEPS.len(), 4);
        assert!(VERIFY_STEPS.iter().any(|step| step.args[0] == "fmt"));
        assert!(VERIFY_STEPS.iter().any(|step| step.args[0] == "check"));
        assert!(VERIFY_STEPS.iter().any(|step| step.args[0] == "clippy"));
        assert!(VERIFY_STEPS.iter().any(|step| step.args[0] == "test"));
        assert!(VERIFY_STEPS.iter().all(|step| !step.name.is_empty()));
    }

    #[test]
    fn lab_gate_targets_only_the_fixture_contracts() {
        assert_eq!(LAB_STEP.args[0], "test");
        assert!(LAB_STEP.args.contains(&"rustit-fixtures"));
        assert!(LAB_STEP.args.contains(&"truth_labs"));
    }
}
