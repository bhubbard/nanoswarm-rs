// ============================================================================
// cli/runner.rs — CLI execution orchestration & terminal rendering
// ============================================================================

use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::cli::args::{CliArgs, Commands};
use crate::conductor::Conductor;
use crate::swarms::{create_audit_swarm, create_dev_swarm, create_triage_swarm};
use crate::tools::ToolRegistry;
use crate::types::{AgentRole, AgentSpec, Severity, SwarmEvent, SwarmReport, Task};

/// Main CLI entrypoint. Dispatches subcommands and manages event display.
pub async fn run_cli(args: CliArgs) -> Result<i32> {
    if !args.json {
        print_banner(&args);
    }

    let engine = apfel::backend::create_engine_with_adapter(
        Some(&args.engine),
        None,
        args.adapter.as_deref(),
    );

    let tools = ToolRegistry::new();

    match args.command {
        Commands::Audit { path, parallel } => {
            let target_path = PathBuf::from(&path);
            let display_path = if target_path.is_relative() {
                format!("./{}", path)
            } else {
                path.clone()
            };

            if !args.json {
                println!(
                    "{} Codebase audit on target: {}\n",
                    "🔍 Launching:".bold().cyan(),
                    display_path.yellow()
                );
            }

            let swarm = create_audit_swarm(engine, tools);
            let task = Task::new(format!(
                "Perform comprehensive security, performance, dead-code, and quality audit on codebase at '{}'",
                display_path
            ));

            let (tx, rx) = mpsc::channel::<SwarmEvent>(64);
            let printer_handle = tokio::spawn(display_events(rx, args.verbose, args.json));

            let report = if parallel {
                swarm.run_parallel(&task, Some(tx)).await?
            } else {
                let mut swarm = swarm;
                swarm.max_turns = args.max_turns;
                swarm.run(&task, Some(tx)).await?
            };

            let _ = printer_handle.await;

            render_report(&report, args.json);
            Ok(
                if report
                    .findings
                    .iter()
                    .any(|f| f.severity == Severity::Critical)
                {
                    1
                } else {
                    0
                },
            )
        }

        Commands::Dev { goal } => {
            if !args.json {
                println!(
                    "{} Autonomous engineering loop for goal: {}\n",
                    "🛠️  Launching:".bold().green(),
                    goal.cyan()
                );
            }

            let mut swarm = create_dev_swarm(engine, tools);
            swarm.max_turns = args.max_turns;

            let task = Task::new(&goal);
            let (tx, rx) = mpsc::channel::<SwarmEvent>(64);
            let printer_handle = tokio::spawn(display_events(rx, args.verbose, args.json));

            let report = swarm.run(&task, Some(tx)).await?;
            let _ = printer_handle.await;

            render_report(&report, args.json);
            Ok(0)
        }

        Commands::Triage => {
            if !args.json {
                println!(
                    "{} Git diff analysis and commit/PR triage\n",
                    "📋 Launching:".bold().magenta()
                );
            }

            let mut swarm = create_triage_swarm(engine, tools);
            swarm.max_turns = args.max_turns;

            let task = Task::new(
                "Analyze unstaged and staged git changes, then propose Conventional Commit messages and a PR summary",
            );
            let (tx, rx) = mpsc::channel::<SwarmEvent>(64);
            let printer_handle = tokio::spawn(display_events(rx, args.verbose, args.json));

            let report = swarm.run(&task, Some(tx)).await?;
            let _ = printer_handle.await;

            render_report(&report, args.json);
            Ok(0)
        }

        Commands::Run { goal, context } => {
            if !args.json {
                println!(
                    "{} Goal-directed swarm: {}\n",
                    "🐝 Launching:".bold().yellow(),
                    goal.cyan()
                );
            }

            let mut conductor = Conductor::new("CustomSwarm", engine, tools);
            conductor.max_turns = args.max_turns;

            // Register standard versatile team
            conductor.register_agent(
                AgentSpec::new(
                    "researcher",
                    AgentRole::Researcher,
                    "Explore codebase, search files, inspect architecture, and synthesize relevant context.",
                )
                .with_tools(vec!["fs_read_file", "fs_search", "fs_list_dir"]),
            );
            conductor.register_agent(
                AgentSpec::new(
                    "architect",
                    AgentRole::Architect,
                    "Break down the problem, design the solution, and formulate verified steps.",
                )
                .with_tools(vec!["fs_read_file"]),
            );
            conductor.register_agent(
                AgentSpec::new(
                    "coder",
                    AgentRole::Coder,
                    "Execute file edits and implementation code accurately.",
                )
                .with_tools(vec!["fs_read_file", "fs_write_file", "fs_search"]),
            );

            let mut task = Task::new(&goal);
            if let Some(ctx) = context {
                task = task.with_context(ctx);
            }

            let (tx, rx) = mpsc::channel::<SwarmEvent>(64);
            let printer_handle = tokio::spawn(display_events(rx, args.verbose, args.json));

            let report = conductor.run(&task, Some(tx)).await?;
            let _ = printer_handle.await;

            render_report(&report, args.json);
            Ok(0)
        }

        Commands::ListSwarms => {
            print_swarm_catalog();
            Ok(0)
        }
    }
}

fn print_banner(args: &CliArgs) {
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════════════".dimmed()
    );
    println!(
        " {} {} {}",
        "🐝".bold(),
        "NanoSwarm".bold().yellow(),
        format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
    );
    println!(
        "   {} | {} | {}",
        format!("Engine: {}", args.engine).cyan(),
        "Router: Zev SIMD (5.8µs)".green(),
        "Cost: $0.00 (100% On-Device)".bold().white()
    );
    println!(
        "{}\n",
        "═══════════════════════════════════════════════════════════════════════".dimmed()
    );
}

async fn display_events(mut rx: mpsc::Receiver<SwarmEvent>, verbose: bool, json_only: bool) {
    if json_only {
        // Drain events silently
        while rx.recv().await.is_some() {}
        return;
    }

    while let Some(event) = rx.recv().await {
        match event {
            SwarmEvent::SwarmStarted { swarm_name, goal } => {
                println!(
                    "{} Swarm [{}] spawned for goal: \"{}\"",
                    "▶".bold().green(),
                    swarm_name.bold().yellow(),
                    goal.white()
                );
            }
            SwarmEvent::AgentAssigned {
                agent_id,
                role,
                subtask,
            } => {
                println!(
                    "  {} [{}] {} assigned → {}",
                    "🤖".cyan(),
                    role.as_str().bold(),
                    agent_id.0.cyan(),
                    subtask.dimmed()
                );
            }
            SwarmEvent::AgentThinking { agent_id, preview } => {
                if verbose {
                    println!(
                        "    💭 {} thinking: {}",
                        agent_id.0.dimmed(),
                        preview.dimmed()
                    );
                }
            }
            SwarmEvent::ToolExecuted {
                agent_id,
                tool,
                input,
                output,
            } => {
                let input_preview: String = input.chars().take(80).collect();
                println!(
                    "    {} {} executed {}({})",
                    "⚙️ ".yellow(),
                    agent_id.0.dimmed(),
                    tool.bold().magenta(),
                    input_preview.dimmed()
                );
                if verbose && !output.is_empty() {
                    let out_preview: String = output.chars().take(120).collect();
                    println!("       ↳ {}", out_preview.dimmed());
                }
            }
            SwarmEvent::GuardrailCheck {
                test,
                passed,
                confidence,
            } => {
                let status = if passed {
                    "PASS".bold().green()
                } else {
                    "FAIL".bold().red()
                };
                println!(
                    "    🛡️  Guardrail [{}]: {} (conf: {:.1}%)",
                    test.dimmed(),
                    status,
                    confidence * 100.0
                );
            }
            SwarmEvent::Handoff { from, to, reason } => {
                println!(
                    "    🔀 Handoff: {} → {} ({})",
                    from.0.cyan(),
                    to.0.cyan(),
                    reason.dimmed()
                );
            }
            SwarmEvent::TaskCompleted { agent_id, summary } => {
                println!(
                    "  {} {} finalized: {}",
                    "✅".green(),
                    agent_id.0.bold(),
                    summary.green()
                );
            }
            SwarmEvent::SwarmFinished {
                duration_ms,
                total_steps,
            } => {
                println!(
                    "{} Swarm finished in {}ms across {} steps\n",
                    "🏁".bold(),
                    duration_ms.to_string().bold().yellow(),
                    total_steps.to_string().bold().cyan()
                );
            }
            SwarmEvent::AgentOutput { agent_id, content } => {
                if verbose {
                    println!("\n--- [Output: {}] ---\n{}\n", agent_id.0.cyan(), content);
                }
            }
            SwarmEvent::Error { message } => {
                eprintln!("  {} {}", "❌ Error:".bold().red(), message.red());
            }
        }
    }
}

fn render_report(report: &SwarmReport, json_output: bool) {
    if json_output {
        if let Ok(json) = serde_json::to_string_pretty(report) {
            println!("{}", json);
        }
        return;
    }

    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════════════".dimmed()
    );
    println!(" {}", "SWARM EXECUTION REPORT".bold().underline());
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════════════".dimmed()
    );
    println!("Swarm:       {}", report.swarm_name.bold().yellow());
    println!("Duration:    {}ms", report.duration_ms.to_string().bold());
    println!("Total Steps: {}", report.steps_executed.to_string().bold());

    print!("Agents:      ");
    for (agent, count) in &report.agent_contributions {
        print!("{}: {} steps  ", agent.cyan(), count.to_string().bold());
    }
    println!();

    if !report.findings.is_empty() {
        println!("\n{}", "FINDINGS & AUDIT RESULTS:".bold().yellow());
        for (i, finding) in report.findings.iter().enumerate() {
            let badge = match finding.severity {
                Severity::Critical => "[CRITICAL]".bold().red(),
                Severity::High => "[HIGH]".bold().bright_red(),
                Severity::Medium => "[MEDIUM]".bold().yellow(),
                Severity::Low => "[LOW]".bold().blue(),
                Severity::Info => "[INFO]".bold().cyan(),
            };
            println!("  {}. {} {}", i + 1, badge, finding.title);
            if let Some(ref path) = finding.file_path {
                println!("     Location: {}", path.dimmed());
            }
            if let Some(ref rec) = finding.recommendation {
                println!("     Recommendation: {}", rec.italic());
            }
        }
    }

    println!("\n{}", "FINAL SUMMARY & ARTIFACTS:".bold().green());
    println!("{}\n", report.summary);
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════════════".dimmed()
    );
}

fn print_swarm_catalog() {
    println!(
        "{}",
        "Built-in NanoSwarm Archetypes & Swarm Templates:"
            .bold()
            .underline()
    );
    println!();

    println!(
        "  {} ({})",
        "nanoswarm audit [PATH]".bold().cyan(),
        "Fan-out or sequential".dimmed()
    );
    println!("    • Security Auditor:     OWASP, injections, secret leaks, path traversal");
    println!("    • Performance Auditor:  Heap allocations, lock contention, hot-loop bottlenecks");
    println!("    • Dead Code Hunter:     Orphan items, unreferenced functions, dead modules");
    println!("    • Quality Auditor:      Error handling coverage, panic prevention, invariants");
    println!();

    println!(
        "  {} ({})",
        "nanoswarm dev \"<GOAL>\"".bold().green(),
        "Autonomous loop".dimmed()
    );
    println!("    • Architect:            Contract design, module decomposition, test plans");
    println!(
        "    • Coder:                Implementation with local tool execution (fs_write_file)"
    );
    println!("    • Verifier:             Test runner & diagnostics (shell_run)");
    println!();

    println!(
        "  {} ({})",
        "nanoswarm triage".bold().magenta(),
        "Git & PR assistant".dimmed()
    );
    println!("    • Diff Analyst:         Inspect staged & unstaged git tree changes");
    println!(
        "    • Release Engineer:     Generate Conventional Commits and GitHub PR descriptions"
    );
    println!();

    println!(
        "  {} ({})",
        "nanoswarm run \"<GOAL>\"".bold().yellow(),
        "Dynamic swarm".dimmed()
    );
    println!("    • Assembles a multi-agent team with Zev sub-microsecond routing");
    println!();
}
