use crate::tracking;
use anyhow::{Context, Result};
use std::process::Command;

pub fn run(args: &[String], verbose: u8) -> Result<()> {
    let timer = tracking::TimedExecution::start();

    // Detect subcommand
    let subcommand = args.first().map(|s| s.as_str());

    let mut cmd = Command::new("supabase");
    for arg in args {
        cmd.arg(arg);
    }

    if verbose > 0 {
        eprintln!("Running: supabase {}", args.join(" "));
    }

    let output = cmd.output().context("Failed to run supabase")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = format!("{}\n{}", stdout, stderr);

    let filtered = match subcommand {
        Some("start") => filter_supabase_start(&raw),
        Some("stop") => filter_supabase_stop(&raw),
        Some("status") => filter_supabase_status(&raw),
        Some("db") => filter_supabase_db(&raw, &args[1..]),
        Some("functions") => filter_supabase_functions(&raw, &args[1..]),
        Some("gen") => filter_supabase_gen(&raw),
        Some("link") => filter_supabase_link(&raw),
        Some("secrets") => filter_supabase_secrets(&raw),
        _ => raw.clone(), // Passthrough for other commands
    };

    println!("{}", filtered.trim());

    timer.track(
        &format!("supabase {}", args.join(" ")),
        &format!("rtk supabase {}", args.join(" ")),
        &raw,
        &filtered,
    );

    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }

    Ok(())
}

/// Filter supabase start - show only essential info and keys
fn filter_supabase_start(output: &str) -> String {
    let mut result = Vec::new();
    let mut found_keys = false;

    for line in output.lines() {
        // Skip verbose container startup
        if line.contains("Starting container")
            || line.contains("Container")
            || line.contains("Seeding data")
            || line.contains("Loading...")
            || line.contains("Applying migration") {
            continue;
        }

        // Keep essential info
        if line.contains("Started supabase")
            || line.contains("API URL:")
            || line.contains("DB URL:")
            || line.contains("Studio URL:")
            || line.contains("anon key:")
            || line.contains("service_role key:") {
            result.push(line.to_string());
            found_keys = true;
        }

        // Keep error messages
        if line.contains("ERROR") || line.contains("Error") || line.contains("Failed") {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓ Supabase started".to_string()
    } else if found_keys {
        // Summarize keys for security
        let summary: Vec<String> = result.iter().map(|line| {
            if line.contains("anon key:") || line.contains("service_role key:") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    format!("{}: {}...", parts[0], &parts[1].trim()[..20.min(parts[1].trim().len())])
                } else {
                    line.clone()
                }
            } else {
                line.clone()
            }
        }).collect();
        summary.join("\n")
    } else {
        result.join("\n")
    }
}

/// Filter supabase stop - simple confirmation
fn filter_supabase_stop(output: &str) -> String {
    for line in output.lines() {
        if line.contains("Stopped supabase") || line.contains("stopped") {
            return "ok ✓ Supabase stopped".to_string();
        }
        if line.contains("ERROR") || line.contains("Error") {
            return line.to_string();
        }
    }

    if output.trim().is_empty() {
        "ok ✓ Supabase stopped".to_string()
    } else {
        output.to_string()
    }
}

/// Filter supabase status - compact table format
fn filter_supabase_status(output: &str) -> String {
    let mut result = Vec::new();
    let mut in_table = false;

    for line in output.lines() {
        // Skip empty lines and separators
        if line.trim().is_empty() || line.chars().all(|c| c == '-' || c == ' ') {
            continue;
        }

        // Keep table headers and content
        if line.contains("SERVICE")
            || line.contains("RUNNING")
            || line.contains("API URL")
            || line.contains("DB URL") {
            result.push(line.to_string());
            in_table = true;
            continue;
        }

        // Keep table rows
        if in_table && (line.starts_with("│") || line.contains("│")) {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "No services running".to_string()
    } else {
        result.join("\n")
    }
}

/// Filter supabase db commands (push, reset, lint, diff)
fn filter_supabase_db(output: &str, args: &[String]) -> String {
    let subcommand = args.first().map(|s| s.as_str());

    match subcommand {
        Some("push") => filter_db_push(output),
        Some("reset") => filter_db_reset(output),
        Some("lint") => filter_db_lint(output),
        Some("diff") => filter_db_diff(output),
        _ => output.to_string(),
    }
}

fn filter_db_push(output: &str) -> String {
    let mut result = Vec::new();
    let mut migration_count = 0;

    for line in output.lines() {
        if line.contains("Applying migration") {
            migration_count += 1;
            continue;
        }

        if line.contains("Applied")
            || line.contains("Finished")
            || line.contains("ERROR")
            || line.contains("Warning") {
            result.push(line.to_string());
        }
    }

    if migration_count > 0 {
        result.insert(0, format!("✓ Applied {} migrations", migration_count));
    }

    if result.is_empty() {
        "ok ✓ Database up to date".to_string()
    } else {
        result.join("\n")
    }
}

fn filter_db_reset(output: &str) -> String {
    for line in output.lines() {
        if line.contains("Finished") || line.contains("Reset") {
            return "ok ✓ Database reset complete".to_string();
        }
        if line.contains("ERROR") {
            return line.to_string();
        }
    }
    "ok ✓ Database reset".to_string()
}

fn filter_db_lint(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        if line.contains("ERROR") || line.contains("Warning") || line.contains("issue") {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓ No lint issues".to_string()
    } else {
        result.join("\n")
    }
}

fn filter_db_diff(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip verbose schema details, keep SQL changes
        if line.starts_with("CREATE")
            || line.starts_with("ALTER")
            || line.starts_with("DROP")
            || line.starts_with("--")
            || line.contains("ERROR") {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓ No schema changes".to_string()
    } else {
        result.join("\n")
    }
}

/// Filter supabase functions commands (deploy, serve)
fn filter_supabase_functions(output: &str, args: &[String]) -> String {
    let subcommand = args.first().map(|s| s.as_str());

    match subcommand {
        Some("deploy") => filter_functions_deploy(output),
        Some("serve") => filter_functions_serve(output),
        _ => output.to_string(),
    }
}

fn filter_functions_deploy(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        if line.contains("Deploying")
            || line.contains("Deployed")
            || line.contains("✓")
            || line.contains("ERROR")
            || line.contains("Failed") {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "ok ✓ Functions deployed".to_string()
    } else {
        result.join("\n")
    }
}

fn filter_functions_serve(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip verbose startup logs
        if line.contains("Serving functions")
            || line.contains("Functions:")
            || line.contains("ERROR")
            || line.contains("Failed") {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        "Functions server started".to_string()
    } else {
        result.join("\n")
    }
}

/// Filter supabase gen types
fn filter_supabase_gen(output: &str) -> String {
    for line in output.lines() {
        if line.contains("Generated") || line.contains("types") {
            // Extract type count if possible
            return "ok ✓ Types generated".to_string();
        }
        if line.contains("ERROR") {
            return line.to_string();
        }
    }

    if output.trim().is_empty() {
        "ok ✓ Types generated".to_string()
    } else {
        output.to_string()
    }
}

/// Filter supabase link
fn filter_supabase_link(output: &str) -> String {
    for line in output.lines() {
        if line.contains("Linked") || line.contains("linked to") {
            return "ok ✓ Project linked".to_string();
        }
        if line.contains("ERROR") || line.contains("Error") {
            return line.to_string();
        }
    }
    output.to_string()
}

/// Filter supabase secrets
fn filter_supabase_secrets(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Don't expose secret values
        if line.contains("Set secret") || line.contains("Updated") {
            result.push("ok ✓ Secret updated".to_string());
        }
        if line.contains("ERROR") {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        output.to_string()
    } else {
        result.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_supabase_start() {
        let output = r#"
Starting container supabase_db_api...
Container supabase_db_api created
Seeding data supabase/seed.sql...
Started supabase local development setup.

         API URL: http://127.0.0.1:54321
          DB URL: postgresql://postgres:postgres@localhost:54322/postgres
      Studio URL: http://127.0.0.1:54323
        anon key: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
service_role key: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
"#;
        let result = filter_supabase_start(output);
        assert!(result.contains("API URL:"));
        assert!(result.contains("anon key:"));
        assert!(!result.contains("Starting container"));
    }

    #[test]
    fn test_filter_supabase_status() {
        let output = r#"
supabase local development setup is running.

         API URL: http://127.0.0.1:54321
          DB URL: postgresql://postgres:postgres@localhost:54322/postgres
      Studio URL: http://127.0.0.1:54323

SERVICE            RUNNING
supabase_db_api    Yes
supabase_kong      Yes
"#;
        let result = filter_supabase_status(output);
        assert!(result.contains("SERVICE"));
        assert!(result.contains("RUNNING"));
    }

    #[test]
    fn test_filter_db_push() {
        let output = r#"
Applying migration 20240101_create_tables.sql...
Applying migration 20240102_add_indexes.sql...
Applied 2 migrations
Finished supabase db push
"#;
        let result = filter_db_push(output);
        assert!(result.contains("Applied 2 migrations"));
        assert!(!result.contains("Applying migration"));
    }

    #[test]
    fn test_filter_functions_deploy() {
        let output = r#"
Deploying function auth...
Deploying function games...
Deployed auth (v2)
Deployed games (v1)
"#;
        let result = filter_functions_deploy(output);
        assert!(result.contains("Deployed"));
        assert!(!result.contains("verbose output"));
    }
}
