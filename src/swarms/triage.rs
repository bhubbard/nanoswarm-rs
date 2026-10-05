// ============================================================================
// swarms/triage.rs — Git & PR Triage Swarm
// ============================================================================

use apfel::backend::BackendEngine;
use std::sync::Arc;

use crate::conductor::Conductor;
use crate::tools::ToolRegistry;
use crate::types::{AgentRole, AgentSpec};

/// Creates a git triage swarm for diff analysis, conventional commit generation, and PR summaries.
pub fn create_triage_swarm(engine: Arc<dyn BackendEngine>, tools: ToolRegistry) -> Conductor {
    let mut conductor = Conductor::new("GitTriageSwarm", engine, tools);

    conductor.register_agent(
        AgentSpec::new(
            "diff_analyst",
            AgentRole::Researcher,
            "You are a Git Diff Analyst. Inspect staged and unstaged changes via git_diff. \
             Categorize modifications across feature additions, refactors, bugfixes, and tests.",
        )
        .with_tools(vec!["git_diff", "shell_run", "fs_read_file"]),
    );

    conductor.register_agent(
        AgentSpec::new(
            "release_engineer",
            AgentRole::Reviewer,
            "You are a Release Engineer. Generate concise Conventional Commit messages (feat, fix, docs, refactor) \
             and a GitHub PR description with breaking change notices, testing steps, and summary bullets."
        )
        .with_tools(vec!["git_diff", "fs_read_file"])
    );

    conductor
}
