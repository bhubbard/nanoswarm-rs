// ============================================================================
// swarms/dev.rs — Autonomous Developer Swarm Loop
// ============================================================================

use apfel::backend::BackendEngine;
use std::sync::Arc;

use crate::conductor::Conductor;
use crate::tools::ToolRegistry;
use crate::types::{AgentRole, AgentSpec};

/// Creates an autonomous software engineering loop (Architect -> Coder -> Verifier).
pub fn create_dev_swarm(engine: Arc<dyn BackendEngine>, tools: ToolRegistry) -> Conductor {
    let mut conductor = Conductor::new("AutonomousDevSwarm", engine, tools);

    conductor.register_agent(
        AgentSpec::new(
            "architect",
            AgentRole::Architect,
            "You are a Software Architect. Break down the user's goal into concrete implementation steps, \
             identify affected modules, plan interface contracts, and specify test criteria."
        )
        .with_tools(vec!["fs_read_file", "fs_search", "fs_list_dir"])
    );

    conductor.register_agent(
        AgentSpec::new(
            "coder",
            AgentRole::Coder,
            "You are a Senior Systems Programmer. Implement the planned changes with clean, \
             type-safe code. Use fs_write_file to write changes and fs_read_file to verify context. \
             Aim for zero warnings and strict error handling."
        )
        .with_tools(vec!["fs_read_file", "fs_write_file", "fs_search"])
    );

    conductor.register_agent(
        AgentSpec::new(
            "verifier",
            AgentRole::Verifier,
            "You are a Quality Verification Engineer. Run project test commands (cargo test, npm test, etc.) \
             using shell_run. If tests fail, diagnose the root cause and provide exact error lines for the coder to fix."
        )
        .with_tools(vec!["shell_run", "fs_read_file", "git_diff"])
    );

    conductor
}
