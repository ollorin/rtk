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
        Some("run") => filter_deno_run(&raw),
        Some("fmt") => filter_deno_fmt(&raw),
        Some("compile") => filter_deno_compile(&raw),
        Some("bench") => filter_deno_bench(&raw),
        Some("doc") => filter_deno_doc(&raw),
        Some("info") => filter_deno_info(&raw),
        Some("install") => filter_deno_install(&raw),
        Some("upgrade") => filter_deno_upgrade(&raw),
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

/// Filter deno run output - strip startup messages
fn filter_deno_run(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip common startup noise
        if line.contains("Download")
            || line.contains("Check file://")
            || line.starts_with("Compile")
            || (line.contains("Warning") && line.contains("--allow-"))
        {
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

/// Filter deno fmt output - show only changed files or errors
fn filter_deno_fmt(output: &str) -> String {
    let mut result = Vec::new();
    let mut formatted_count = 0;

    for line in output.lines() {
        // Skip "Checked" messages for unchanged files
        if line.contains("Checked") && !line.contains("error") {
            continue;
        }

        // Count formatted files
        if line.contains("Formatted") || line.contains("formatted") {
            formatted_count += 1;
            continue;
        }

        // Keep errors and changed file indicators
        if line.contains("error:")
            || line.contains("Error")
            || line.starts_with("error[")
            || !line.trim().is_empty() && !line.contains("Checking")
        {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        if formatted_count > 0 {
            format!("ok ✓ Formatted {} files", formatted_count)
        } else {
            "ok ✓ No formatting needed".to_string()
        }
    } else {
        result.join("\n")
    }
}

/// Filter deno compile output - show only final binary info
fn filter_deno_compile(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip verbose compilation messages
        if line.contains("Bundle")
            || line.contains("Compile")
            || line.contains("Download")
        {
            continue;
        }

        // Keep binary output info and errors
        if line.contains("Emit")
            || line.contains("emit")
            || line.contains("Wrote")
            || line.contains("error:")
            || line.contains("Error")
        {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓ Binary compiled".to_string()
    } else {
        result.join("\n")
    }
}

/// Filter deno bench output - show summary only
fn filter_deno_bench(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip warmup and individual iteration outputs
        if line.contains("running")
            || line.contains("Warmup")
            || line.contains("warmup")
            || line.starts_with("    at ")
        {
            continue;
        }

        // Keep benchmark results and summary
        if line.contains("benchmark")
            || line.contains("Benchmark")
            || line.contains("time:")
            || line.contains("iter/s")
            || line.contains("result:")
            || line.contains("error:")
        {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓ Benchmarks complete".to_string()
    } else {
        result.join("\n")
    }
}

/// Filter deno doc output - keep structure, trim verbose
fn filter_deno_doc(output: &str) -> String {
    let mut result = Vec::new();
    let mut line_count = 0;
    let max_lines = 100;

    for line in output.lines() {
        // Skip empty lines at start
        if result.is_empty() && line.trim().is_empty() {
            continue;
        }

        // Keep documentation structure
        if !line.trim().is_empty() {
            result.push(line.to_string());
            line_count += 1;

            if line_count >= max_lines {
                result.push(format!("... ({} more lines)", output.lines().count() - max_lines));
                break;
            }
        }
    }

    if result.is_empty() {
        "ok ✓ No documentation generated".to_string()
    } else {
        result.join("\n")
    }
}

/// Filter deno info output - keep essential info only
fn filter_deno_info(output: &str) -> String {
    let mut result = Vec::new();
    let mut in_dependencies = false;
    let mut dep_count = 0;

    for line in output.lines() {
        // Keep version and main info
        if line.starts_with("deno")
            || line.starts_with("local:")
            || line.starts_with("emit:")
            || line.starts_with("type:")
            || line.starts_with("size:")
        {
            result.push(line.to_string());
            continue;
        }

        // Summarize dependencies section
        if line.contains("dependencies:") {
            in_dependencies = true;
            continue;
        }

        if in_dependencies {
            if line.starts_with("  ") && line.contains("http") {
                dep_count += 1;
            }
        }
    }

    if dep_count > 0 {
        result.push(format!("deps: {} modules", dep_count));
    }

    if result.is_empty() {
        output.to_string()
    } else {
        result.join("\n")
    }
}

/// Filter deno install output - strip verbose download info
fn filter_deno_install(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip verbose download messages
        if line.contains("Download")
            || line.starts_with("  ")
            && line.contains("http")
        {
            continue;
        }

        // Keep success/error messages
        if line.contains("Installed")
            || line.contains("installed")
            || line.contains("✓")
            || line.contains("error:")
            || line.contains("Error")
            || line.contains("Warning")
        {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓ Installed".to_string()
    } else {
        result.join("\n")
    }
}

/// Filter deno upgrade output - show version change only
fn filter_deno_upgrade(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip download progress
        if line.contains("Download")
            || line.contains("%")
            || line.contains("...")
        {
            continue;
        }

        // Keep version info
        if line.contains("deno")
            || line.contains("upgraded")
            || line.contains("latest")
            || line.contains("->")
            || line.contains("error:")
        {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓ Deno up to date".to_string()
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
        // Should keep the test result summary line
        assert!(result.contains("test result:") || result.contains("passed"));
        assert!(!result.contains("Check file://"));
        assert!(!result.contains("Download"));
    }

    #[test]
    fn test_filter_deno_test_empty() {
        // When no summary line present, should show default message
        let output = r#"
Check file:///Users/test/app.ts
Download https://deno.land/std@0.224.0/assert/mod.ts
Running 10 tests from app_test.ts
"#;
        let result = filter_deno_test(output);
        assert!(result.contains("ok ✓ All tests passed"));
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

    #[test]
    fn test_filter_deno_run_clean() {
        let output = r#"
Download https://deno.land/std@0.224.0/http/server.ts
Check file:///Users/test/server.ts
Server listening on http://localhost:8000
"#;
        let result = filter_deno_run(output);
        assert!(result.contains("Server listening"));
        assert!(!result.contains("Download"));
        assert!(!result.contains("Check file://"));
    }

    #[test]
    fn test_filter_deno_fmt_clean() {
        let output = "Checked 15 files\n";
        let result = filter_deno_fmt(output);
        assert!(result.contains("ok ✓"));
    }

    #[test]
    fn test_filter_deno_fmt_with_changes() {
        let output = r#"
Formatted src/main.ts
Formatted src/lib.ts
Formatted tests/app_test.ts
"#;
        let result = filter_deno_fmt(output);
        assert!(result.contains("Formatted 3 files"));
    }

    #[test]
    fn test_filter_deno_info() {
        let output = r#"
deno 1.40.0
local: /Users/test/.cache/deno/deps/
type: TypeScript
size: 42KB

dependencies:
  https://deno.land/std@0.224.0/http/server.ts
  https://deno.land/std@0.224.0/fmt/colors.ts
  https://deno.land/x/oak@v12.0.0/mod.ts
"#;
        let result = filter_deno_info(output);
        assert!(result.contains("deno 1.40.0"));
        assert!(result.contains("deps: 3 modules"));
        assert!(!result.contains("https://"));
    }

    #[test]
    fn test_filter_deno_install() {
        let output = r#"
Download https://deno.land/x/denon@2.5.0/mod.ts
Download https://deno.land/std@0.224.0/path/mod.ts
✓ Successfully installed denon
"#;
        let result = filter_deno_install(output);
        assert!(result.contains("✓"));
        assert!(!result.contains("Download"));
    }

    #[test]
    fn test_filter_deno_upgrade() {
        let output = r#"
Downloading https://github.com/denoland/deno/releases/download/v1.41.0/deno-x86_64-apple-darwin.zip
100.0%
deno upgraded from 1.40.0 to 1.41.0
"#;
        let result = filter_deno_upgrade(output);
        assert!(result.contains("deno upgraded"));
        assert!(!result.contains("Downloading"));
        assert!(!result.contains("100.0%"));
    }
}
