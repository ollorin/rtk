use crate::tracking;
use anyhow::{Context, Result};
use std::process::Command;

pub fn run(args: &[String], verbose: u8) -> Result<()> {
    let timer = tracking::TimedExecution::start();

    // Detect subcommand
    let subcommand = args.first().map(|s| s.as_str());

    let mut cmd = Command::new("deno");
    for arg in args {
        cmd.arg(arg);
    }

    if verbose > 0 {
        eprintln!("Running: deno {}", args.join(" "));
    }

    let output = cmd.output().context("Failed to run deno")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = format!("{}\n{}", stdout, stderr);

    let filtered = match subcommand {
        Some("test") => filter_deno_test(&raw),
        Some("lint") => filter_deno_lint(&raw),
        Some("check") => filter_deno_check(&raw),
        Some("task") => filter_deno_task(&raw),
        _ => raw.clone(), // Passthrough for other commands
    };

    println!("{}", filtered.trim());

    timer.track(
        &format!("deno {}", args.join(" ")),
        &format!("rtk deno {}", args.join(" ")),
        &raw,
        &filtered,
    );

    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }

    Ok(())
}

/// Filter deno test output - show only summary and failures
fn filter_deno_test(output: &str) -> String {
    let mut result = Vec::new();
    let mut in_failure = false;
    let mut failure_block = Vec::new();

    for line in output.lines() {
        // Skip verbose startup messages
        if line.contains("Check file://")
            || line.contains("Download")
            || line.contains("Running")
            || line.starts_with("    at ") && !in_failure {
            continue;
        }

        // Detect failure blocks
        if line.contains("FAILED") || line.contains("Error:") || line.contains("AssertionError") {
            in_failure = true;
            failure_block.push(line.to_string());
            continue;
        }

        // Collect failure details
        if in_failure {
            if line.trim().is_empty() {
                in_failure = false;
                result.extend(failure_block.drain(..));
                result.push(String::new());
            } else {
                failure_block.push(line.to_string());
            }
            continue;
        }

        // Keep summary lines
        if line.contains("test result:")
            || line.contains("ok |")
            || line.contains("passed")
            || line.contains("failed")
            || line.starts_with("FAILED") {
            result.push(line.to_string());
        }
    }

    // Add any remaining failure block
    if !failure_block.is_empty() {
        result.extend(failure_block);
    }

    if result.is_empty() {
        "ok ✓ All tests passed".to_string()
    } else {
        result.join("\n")
    }
}

/// Filter deno lint output - show only errors/warnings
fn filter_deno_lint(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip verbose file checks
        if line.contains("Checked") && line.contains("file") {
            continue;
        }

        // Keep errors and warnings
        if line.contains("error:")
            || line.contains("warning:")
            || line.contains("hint:")
            || line.starts_with("Found") {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓ No lint issues".to_string()
    } else {
        result.join("\n")
    }
}

/// Filter deno check output - show only errors
fn filter_deno_check(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip check file messages
        if line.contains("Check file://") {
            continue;
        }

        // Keep errors
        if line.contains("error:")
            || line.contains("TS")
            || line.starts_with("    at ")
            || line.trim().starts_with("^") {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓ Type check passed".to_string()
    } else {
        result.join("\n")
    }
}

/// Filter deno task output - strip task runner boilerplate
fn filter_deno_task(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip task runner messages
        if line.starts_with("Task") && line.contains("deno") {
            continue;
        }

        // Keep actual output
        if !line.trim().is_empty() {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓".to_string()
    } else {
        result.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_deno_test_success() {
        let output = r#"
Check file:///Users/test/app.ts
Download https://deno.land/std@0.224.0/assert/mod.ts
Running 10 tests from app_test.ts

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out (1.2s)
"#;
        let result = filter_deno_test(output);
        assert!(result.contains("ok ✓ All tests passed"));
        assert!(!result.contains("Check file://"));
        assert!(!result.contains("Download"));
    }

    #[test]
    fn test_filter_deno_test_with_failures() {
        let output = r#"
Check file:///Users/test/app.ts
Running 10 tests from app_test.ts

FAILED test_something
Error: Test failed
    at assertEquals (file:///app.ts:42:10)

test result: FAILED. 9 passed; 1 failed; 0 ignored (1.2s)
"#;
        let result = filter_deno_test(output);
        assert!(result.contains("FAILED"));
        assert!(result.contains("Test failed"));
        assert!(!result.contains("Check file://"));
    }

    #[test]
    fn test_filter_deno_lint_clean() {
        let output = "Checked 42 files\n";
        let result = filter_deno_lint(output);
        assert_eq!(result, "ok ✓ No lint issues");
    }

    #[test]
    fn test_filter_deno_lint_with_errors() {
        let output = r#"
Checked 42 files
error: unused variable
warning: prefer const
Found 2 problems
"#;
        let result = filter_deno_lint(output);
        assert!(result.contains("error: unused variable"));
        assert!(result.contains("Found 2 problems"));
        assert!(!result.contains("Checked"));
    }
}
