use crate::tracking;
use anyhow::{Context, Result};
use std::process::Command;

pub fn run(args: &[String], verbose: u8) -> Result<()> {
    let timer = tracking::TimedExecution::start();

    // Detect if this is an npx nx call
    let is_npx = args.first().map(|s| s.as_str()) == Some("nx");

    let mut cmd = if is_npx {
        let mut c = Command::new("npx");
        c.arg("nx");
        for arg in &args[1..] {
            c.arg(arg);
        }
        c
    } else {
        let mut c = Command::new("nx");
        for arg in args {
            c.arg(arg);
        }
        c
    };

    if verbose > 0 {
        eprintln!("Running: {}", if is_npx {
            format!("npx nx {}", args[1..].join(" "))
        } else {
            format!("nx {}", args.join(" "))
        });
    }

    let output = cmd.output().context("Failed to run nx")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = format!("{}\n{}", stdout, stderr);

    let filtered = filter_nx_output(&raw, args);

    println!("{}", filtered.trim());

    let cmd_str = if is_npx {
        format!("npx nx {}", args[1..].join(" "))
    } else {
        format!("nx {}", args.join(" "))
    };

    timer.track(
        &cmd_str,
        &format!("rtk {}", cmd_str),
        &raw,
        &filtered,
    );

    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }

    Ok(())
}

/// Filter Nx output - remove task graph visualization and verbose logs
fn filter_nx_output(output: &str, args: &[String]) -> String {
    let mut result = Vec::new();
    let mut skip_task_graph = false;

    // Detect command type from args
    let is_test = args.iter().any(|a| a == "test" || a == "e2e");
    let is_build = args.iter().any(|a| a == "build");
    let is_serve = args.iter().any(|a| a == "serve" || a == "dev" || a == "start" || a.starts_with("start:"));
    let is_affected = args.iter().any(|a| a == "affected");

    for line in output.lines() {
        // Skip task graph visualization
        if line.contains("Tasks to run for affected projects") || line.starts_with(" >") && line.contains(":") {
            skip_task_graph = true;
            continue;
        }

        // End of task graph
        if skip_task_graph && line.trim().is_empty() {
            skip_task_graph = false;
            continue;
        }

        if skip_task_graph {
            continue;
        }

        // Skip Nx Cloud ads and prompts
        if line.contains("Nx Cloud")
            || line.contains("nx.app")
            || line.contains("faster remote builds")
            || line.contains("run-many")
            || line.contains("NX   Nx ") {
            continue;
        }

        // Skip verbose dependency graph
        if line.starts_with("   - ") && line.contains("[") {
            continue;
        }

        // For serve/dev commands, only keep essential startup info
        if is_serve {
            if line.contains("Application bundle generation complete")
                || line.contains("Compiled successfully")
                || line.contains("Local:")
                || line.contains("ready -")
                || line.contains("started")
                || line.contains("ERROR")
                || line.contains("WARNING") {
                result.push(line.to_string());
            }
            continue;
        }

        // For test commands, show summary
        if is_test {
            if line.contains("PASS")
                || line.contains("FAIL")
                || line.contains("Test Suites:")
                || line.contains("Tests:")
                || line.contains("Snapshots:")
                || line.contains("ERROR") {
                result.push(line.to_string());
            }
            continue;
        }

        // For build commands, show progress and completion
        if is_build {
            if line.contains("Building")
                || line.contains("Compiling")
                || line.contains("Successfully")
                || line.contains("✓")
                || line.contains("ERROR")
                || line.contains("WARNING")
                || line.contains("Bundle")
                || line.contains("Initial Chunk Files") {
                result.push(line.to_string());
            }
            continue;
        }

        // For affected commands, show affected projects
        if is_affected {
            if line.contains("Affected projects:")
                || line.starts_with("  - ")
                || line.contains("NX   Running target") {
                result.push(line.to_string());
            }
            continue;
        }

        // Keep important lines for all commands
        if line.contains("✓")
            || line.contains("✔")
            || line.contains("Successfully")
            || line.contains("ERROR")
            || line.contains("FAILED")
            || line.contains("Warning")
            || line.starts_with("NX   Successfully ran target")
            || line.starts_with("NX   Ran target") {
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
    fn test_filter_nx_test_output() {
        let output = r#"
NX   Running target test for project api

Tasks to run for affected projects:
 > api:test

PASS  apps/api/test/app.test.ts
Test Suites: 1 passed, 1 total
Tests:       5 passed, 5 total

NX   Successfully ran target test for project api
"#;
        let args = vec!["test".to_string(), "api".to_string()];
        let result = filter_nx_output(output, &args);
        assert!(result.contains("PASS"));
        assert!(result.contains("Test Suites:"));
        assert!(!result.contains("Tasks to run"));
    }

    #[test]
    fn test_filter_nx_build_output() {
        let output = r#"
NX   Running target build for project player-web

Building player-web...
✓ Compiled successfully
Bundle size: 245 kB

NX   Successfully ran target build
"#;
        let args = vec!["build".to_string(), "player-web".to_string()];
        let result = filter_nx_output(output, &args);
        assert!(result.contains("✓ Compiled successfully"));
        assert!(result.contains("Bundle"));
    }

    #[test]
    fn test_filter_nx_affected() {
        let output = r#"
NX   Affected projects:

  - api
  - player-web
  - operator-web

NX   Running target test for 3 projects
"#;
        let args = vec!["affected:test".to_string()];
        let result = filter_nx_output(output, &args);
        assert!(result.contains("- api"));
        assert!(result.contains("- player-web"));
    }
}
