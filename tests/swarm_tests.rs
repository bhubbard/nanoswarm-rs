// ============================================================================
// tests/swarm_tests.rs — Comprehensive integration tests for NanoSwarm
// ============================================================================

use std::sync::Arc;
use tempfile::tempdir;

use nanoswarm::conductor::SwarmRouter;
use nanoswarm::swarms::{create_audit_swarm, create_dev_swarm, create_triage_swarm};
use nanoswarm::tools::ToolRegistry;
use nanoswarm::types::{AgentRole, AgentSpec, Task};

#[tokio::test]
async fn test_tools_file_operations_and_sandboxing() {
    let dir = tempdir().expect("create tempdir");
    let dir_path = dir.path().to_path_buf();

    let tools = ToolRegistry::with_root(&dir_path);

    // 1. Write file
    let write_res = tools
        .execute(
            "fs_write_file",
            &serde_json::json!({
                "path": "src/hello.rs",
                "content": "pub fn hello() -> &'static str { \"world\" }\n// [CRITICAL] test secret leaked"
            }),
        )
        .await
        .expect("write file");
    assert!(write_res.contains("Successfully wrote"));

    // 2. Read file
    let read_res = tools
        .execute(
            "fs_read_file",
            &serde_json::json!({ "path": "src/hello.rs" }),
        )
        .await
        .expect("read file");
    assert!(read_res.contains("pub fn hello()"));

    // 3. List directory
    let list_res = tools
        .execute("fs_list_dir", &serde_json::json!({ "path": "." }))
        .await
        .expect("list dir");
    assert!(list_res.contains("src"));

    // 4. Search files
    let search_res = tools
        .execute(
            "fs_search",
            &serde_json::json!({
                "path": ".",
                "pattern": "CRITICAL"
            }),
        )
        .await
        .expect("search files");
    assert!(search_res.contains("[CRITICAL]"));

    // 5. Sandboxing escape attempt must fail
    let escape_res = tools
        .execute(
            "fs_read_file",
            &serde_json::json!({ "path": "../../outside.txt" }),
        )
        .await;
    assert!(
        escape_res.is_err(),
        "Escaping root directory must be rejected"
    );
}

#[tokio::test]
async fn test_tools_shell_run() {
    let tools = ToolRegistry::new();
    let res = tools
        .execute(
            "shell_run",
            &serde_json::json!({
                "command": "echo 'nanoswarm-test-output'",
                "timeout_secs": 5
            }),
        )
        .await
        .expect("shell run");
    assert!(res.contains("nanoswarm-test-output"));
}

#[tokio::test]
async fn test_audit_swarm_parallel_fanout() {
    let mock_engine = Arc::new(apfel::backend::MockEngine::new());
    let tools = ToolRegistry::new();

    let swarm = create_audit_swarm(mock_engine, tools);
    assert_eq!(swarm.agents.len(), 4);

    let task = Task::new("Audit codebase for vulnerabilities and allocations");
    let report = swarm
        .run_parallel(&task, None)
        .await
        .expect("parallel audit");

    assert_eq!(report.steps_executed, 4);
    assert_eq!(report.agent_contributions.len(), 4);
    assert!(!report.summary.is_empty());
}

#[tokio::test]
async fn test_dev_swarm_sequential_loop() {
    let mock_engine = Arc::new(apfel::backend::MockEngine::new());
    let tools = ToolRegistry::new();

    let mut swarm = create_dev_swarm(mock_engine, tools);
    swarm.max_turns = 2;

    let task = Task::new("Refactor authentication module");
    let report = swarm.run(&task, None).await.expect("dev swarm run");

    assert!(report.steps_executed >= 1);
    assert_eq!(report.goal, "Refactor authentication module");
}

#[tokio::test]
async fn test_triage_swarm() {
    let mock_engine = Arc::new(apfel::backend::MockEngine::new());
    let tools = ToolRegistry::new();

    let mut swarm = create_triage_swarm(mock_engine, tools);
    swarm.max_turns = 2;

    let task = Task::new("Triage recent git diff");
    let report = swarm.run(&task, None).await.expect("triage run");

    assert!(report.steps_executed >= 1);
}

#[tokio::test]
async fn test_router_agent_selection() {
    let agents = vec![
        AgentSpec::new(
            "sec",
            AgentRole::SecurityAuditor,
            "Security vulnerability analysis, OWASP, SQL injection",
        ),
        AgentSpec::new(
            "perf",
            AgentRole::PerformanceAuditor,
            "Low-level memory allocations, cache locality, SIMD throughput",
        ),
        AgentSpec::new(
            "hygiene",
            AgentRole::DeadCodeHunter,
            "Unused dependencies, dead functions, orphan modules",
        ),
    ];

    let sec_pick = SwarmRouter::select_best_agent("Fix critical SQL injection flaw", &agents);
    assert_eq!(sec_pick.unwrap().id.0, "sec");

    let perf_pick =
        SwarmRouter::select_best_agent("Optimize heap allocations in inner loop", &agents);
    assert_eq!(perf_pick.unwrap().id.0, "perf");
}
