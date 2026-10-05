// ============================================================================
// swarms/audit.rs — Specialized Codebase Audit Swarm
// ============================================================================

use apfel::backend::BackendEngine;
use std::sync::Arc;

use crate::conductor::Conductor;
use crate::tools::ToolRegistry;
use crate::types::{AgentRole, AgentSpec};

/// Creates a specialized 4-agent security & performance audit swarm.
pub fn create_audit_swarm(engine: Arc<dyn BackendEngine>, tools: ToolRegistry) -> Conductor {
    let mut conductor = Conductor::new("CodeAuditSwarm", engine, tools);

    conductor.register_agent(
        AgentSpec::new(
            "sec_auditor",
            AgentRole::SecurityAuditor,
            "You are a Senior Application Security Auditor. Inspect code for OWASP vulnerabilities, \
             hardcoded secrets/tokens, injection flaws, unsafe memory blocks, and path traversal vulnerabilities. \
             Prefix severe issues with [CRITICAL], [HIGH], or [MEDIUM]."
        )
        .with_tools(vec!["fs_read_file", "fs_search", "fs_list_dir"])
    );

    conductor.register_agent(
        AgentSpec::new(
            "perf_auditor",
            AgentRole::PerformanceAuditor,
            "You are a Systems Performance Engineer. Audit code for unnecessary heap allocations, \
             unbounded loops, blocking calls inside async runtimes, redundant clone operations, and lock contention. \
             Suggest high-performance zero-copy alternatives."
        )
        .with_tools(vec!["fs_read_file", "fs_search"])
    );

    conductor.register_agent(
        AgentSpec::new(
            "dead_code_hunter",
            AgentRole::DeadCodeHunter,
            "You are a Code Hygiene Specialist. Detect unused dependencies, orphan functions, \
             commented-out dead blocks, redundant abstractions, and unreferenced exports.",
        )
        .with_tools(vec!["fs_read_file", "fs_search"]),
    );

    conductor.register_agent(
        AgentSpec::new(
            "quality_auditor",
            AgentRole::QualityAuditor,
            "You are a Software Quality & Verification Lead. Review error handling completeness, \
             panic-safety, test coverage holes, and documentation clarity.",
        )
        .with_tools(vec!["fs_read_file", "fs_search"]),
    );

    conductor
}
