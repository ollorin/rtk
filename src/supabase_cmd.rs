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
        Some("migration") => filter_supabase_migration(&raw, &args[1..]),
        Some("inspect") => filter_supabase_inspect(&raw, &args[1..]),
        Some("test") => filter_supabase_test(&raw),
        Some("projects") => filter_supabase_projects(&raw),
        Some("branches") => filter_supabase_branches(&raw),
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

/// Filter supabase migration commands (list, new, up, repair)
fn filter_supabase_migration(output: &str, args: &[String]) -> String {
    let subcommand = args.first().map(|s| s.as_str());

    match subcommand {
        Some("list") => filter_migration_list(output),
        Some("new") => filter_migration_new(output),
        Some("up") => filter_migration_up(output),
        Some("repair") => filter_migration_repair(output),
        _ => output.to_string(),
    }
}

fn filter_migration_list(output: &str) -> String {
    let mut result = Vec::new();
    let mut pending_count = 0;
    let mut applied_count = 0;

    for line in output.lines() {
        // Skip empty lines and separators
        if line.trim().is_empty() || line.chars().all(|c| c == '-' || c == '=' || c == ' ') {
            continue;
        }

        // Count migrations by status
        if line.contains("local") || line.contains("pending") {
            pending_count += 1;
        } else if line.contains("applied") {
            applied_count += 1;
        }

        // Keep migration names (simplified)
        if line.contains(".sql") || line.starts_with("20") {
            // Extract just the name part if possible
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                result.push(parts[0].to_string());
            }
        }
    }

    let summary = format!("Migrations: {} applied, {} pending", applied_count, pending_count);

    if result.is_empty() {
        if applied_count > 0 || pending_count > 0 {
            summary
        } else {
            "No migrations found".to_string()
        }
    } else if result.len() > 10 {
        // Truncate long lists
        format!("{}\n{} total migrations ({} shown)",
            result[..5].join("\n"),
            result.len(),
            5)
    } else {
        format!("{}\n{}", result.join("\n"), summary)
    }
}

fn filter_migration_new(output: &str) -> String {
    for line in output.lines() {
        if line.contains("Created") || line.contains("created") {
            return line.to_string();
        }
        if line.contains("ERROR") || line.contains("Error") {
            return line.to_string();
        }
    }
    "ok ✓ Migration created".to_string()
}

fn filter_migration_up(output: &str) -> String {
    let mut result = Vec::new();
    let mut migration_count = 0;

    for line in output.lines() {
        if line.contains("Applying") || line.contains("Running") {
            migration_count += 1;
            continue;
        }

        if line.contains("Applied")
            || line.contains("Finished")
            || line.contains("ERROR")
            || line.contains("Error") {
            result.push(line.to_string());
        }
    }

    if migration_count > 0 && result.is_empty() {
        format!("ok ✓ Applied {} migrations", migration_count)
    } else if result.is_empty() {
        "ok ✓ No pending migrations".to_string()
    } else {
        result.join("\n")
    }
}

fn filter_migration_repair(output: &str) -> String {
    for line in output.lines() {
        if line.contains("Repaired") || line.contains("repaired") || line.contains("Fixed") {
            return "ok ✓ Migration history repaired".to_string();
        }
        if line.contains("ERROR") || line.contains("Error") {
            return line.to_string();
        }
    }
    "ok ✓ Repair complete".to_string()
}

/// Filter supabase inspect commands (db, bloat, etc.)
fn filter_supabase_inspect(output: &str, args: &[String]) -> String {
    let subcommand = args.first().map(|s| s.as_str());

    match subcommand {
        Some("db") => filter_inspect_db(output),
        _ => output.to_string(),
    }
}

fn filter_inspect_db(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Keep database info summaries
        if line.contains("Size:")
            || line.contains("Tables:")
            || line.contains("Indexes:")
            || line.contains("Total")
            || line.contains("ERROR") {
            result.push(line.to_string());
        }
    }

    if result.is_empty() {
        output.to_string()
    } else {
        result.join("\n")
    }
}

/// Filter supabase test output
fn filter_supabase_test(output: &str) -> String {
    let mut result = Vec::new();
    let mut pass_count = 0;
    let mut fail_count = 0;

    for line in output.lines() {
        // Count test results
        if line.contains("✓") || line.contains("PASS") {
            pass_count += 1;
            continue;
        }
        if line.contains("✗") || line.contains("FAIL") {
            fail_count += 1;
            result.push(line.to_string());
            continue;
        }

        // Keep error details
        if line.contains("ERROR") || line.contains("Error:") {
            result.push(line.to_string());
        }
    }

    let summary = if fail_count > 0 {
        format!("Tests: {} passed, {} FAILED", pass_count, fail_count)
    } else if pass_count > 0 {
        format!("ok ✓ {} tests passed", pass_count)
    } else {
        "ok ✓ Tests complete".to_string()
    };

    if result.is_empty() {
        summary
    } else {
        format!("{}\n{}", summary, result.join("\n"))
    }
}

/// Filter supabase projects list
fn filter_supabase_projects(output: &str) -> String {
    let mut result = Vec::new();
    let mut project_count = 0;

    for line in output.lines() {
        // Skip verbose table formatting
        if line.chars().all(|c| c == '-' || c == '+' || c == ' ' || c == '|') {
            continue;
        }

        // Keep project names and IDs
        if line.contains("│") {
            let parts: Vec<&str> = line.split('│').collect();
            if parts.len() >= 2 {
                let name = parts[1].trim();
                if !name.is_empty() && !name.to_lowercase().contains("name") {
                    result.push(name.to_string());
                    project_count += 1;
                }
            }
        }
    }

    if result.is_empty() {
        if output.contains("No projects") {
            "No projects found".to_string()
        } else {
            output.to_string()
        }
    } else {
        format!("{} projects:\n{}", project_count, result.join("\n"))
    }
}

/// Filter supabase branches output
fn filter_supabase_branches(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        // Skip verbose formatting
        if line.chars().all(|c| c == '-' || c == '+' || c == ' ') {
            continue;
        }

        // Keep branch info
        if line.contains("main")
            || line.contains("preview")
            || line.contains("*")  // Current branch marker
            || line.contains("branch")
        {
            result.push(line.trim().to_string());
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

    #[test]
    fn test_filter_migration_list() {
        let output = r#"
LOCAL      REMOTE    NAME
---------  --------  ---------------------------------
applied    applied   20240101000000_initial.sql
applied    applied   20240102000000_add_players.sql
local      pending   20240103000000_add_games.sql
"#;
        let result = filter_migration_list(output);
        assert!(result.contains("applied"));
        assert!(result.contains("pending"));
    }

    #[test]
    fn test_filter_migration_new() {
        let output = "Created supabase/migrations/20240215000000_add_wallets.sql\n";
        let result = filter_migration_new(output);
        assert!(result.contains("Created"));
    }

    #[test]
    fn test_filter_migration_up() {
        let output = r#"
Applying migration 20240103000000_add_games.sql...
Running migration...
Applied migration 20240103000000_add_games.sql
Finished db push
"#;
        let result = filter_migration_up(output);
        assert!(result.contains("Applied"));
        assert!(!result.contains("Applying"));
    }

    #[test]
    fn test_filter_supabase_test() {
        let output = r#"
Running pgTAP tests...
✓ test_player_insert
✓ test_player_update
✗ test_player_delete - ERROR: permission denied
"#;
        let result = filter_supabase_test(output);
        assert!(result.contains("passed"));
        assert!(result.contains("FAILED"));
        assert!(result.contains("permission denied"));
    }

    #[test]
    fn test_filter_supabase_test_all_pass() {
        let output = r#"
Running pgTAP tests...
✓ test_player_insert
✓ test_player_update
✓ test_player_delete
All tests passed!
"#;
        let result = filter_supabase_test(output);
        assert!(result.contains("ok ✓"));
        assert!(result.contains("3 tests passed"));
    }
}
