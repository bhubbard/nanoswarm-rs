//! NanoSwarm: 100% On-Device Autonomous AI Agent Swarm in Native Rust.

pub mod agent;
pub mod cli;
pub mod conductor;
pub mod swarms;
pub mod tools;
pub mod types;

pub use agent::Agent;
pub use conductor::{Conductor, SwarmRouter};
pub use tools::ToolRegistry;
pub use types::{
    AgentId, AgentRole, AgentSpec, Finding, Severity, SwarmEvent, SwarmReport, Task, TaskStatus,
};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
