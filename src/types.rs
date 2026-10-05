// ============================================================================
// types.rs — Core data structures for NanoSwarm
// ============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for an agent instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Specialized archetypal roles for swarm members.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    Conductor,
    Architect,
    Coder,
    SecurityAuditor,
    PerformanceAuditor,
    DeadCodeHunter,
    QualityAuditor,
    Verifier,
    Researcher,
    Reviewer,
    Custom(String),
}

impl AgentRole {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Conductor => "Conductor",
            Self::Architect => "Architect",
            Self::Coder => "Coder",
            Self::SecurityAuditor => "Security Auditor",
            Self::PerformanceAuditor => "Performance Auditor",
            Self::DeadCodeHunter => "Dead Code Hunter",
            Self::QualityAuditor => "Quality Auditor",
            Self::Verifier => "Verifier",
            Self::Researcher => "Researcher",
            Self::Reviewer => "Reviewer",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// Specification defining an individual agent within a swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub id: AgentId,
    pub role: AgentRole,
    pub system_prompt: String,
    pub tools: Vec<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<usize>,
}

impl AgentSpec {
    pub fn new(id: impl Into<String>, role: AgentRole, system_prompt: impl Into<String>) -> Self {
        Self {
            id: AgentId::new(id),
            role,
            system_prompt: system_prompt.into(),
            tools: Vec::new(),
            temperature: Some(0.3),
            max_tokens: Some(1024),
        }
    }

    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tools.push(tool.into());
        self
    }

    pub fn with_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for t in tools {
            self.tools.push(t.into());
        }
        self
    }
}

/// A discrete unit of work passed to an agent or swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub goal: String,
    pub context: String,
    pub status: TaskStatus,
    pub artifacts: HashMap<String, String>,
}

impl Task {
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            id: uuid_simple(),
            goal: goal.into(),
            context: String::new(),
            status: TaskStatus::Pending,
            artifacts: HashMap::new(),
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

/// Events emitted during swarm execution for live telemetry and UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwarmEvent {
    SwarmStarted {
        swarm_name: String,
        goal: String,
    },
    AgentAssigned {
        agent_id: AgentId,
        role: AgentRole,
        subtask: String,
    },
    AgentThinking {
        agent_id: AgentId,
        preview: String,
    },
    AgentOutput {
        agent_id: AgentId,
        content: String,
    },
    ToolExecuted {
        agent_id: AgentId,
        tool: String,
        input: String,
        output: String,
    },
    GuardrailCheck {
        test: String,
        passed: bool,
        confidence: f64,
    },
    Handoff {
        from: AgentId,
        to: AgentId,
        reason: String,
    },
    TaskCompleted {
        agent_id: AgentId,
        summary: String,
    },
    SwarmFinished {
        duration_ms: u64,
        total_steps: usize,
    },
    Error {
        message: String,
    },
}

/// Comprehensive report generated at the conclusion of a swarm run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmReport {
    pub goal: String,
    pub swarm_name: String,
    pub duration_ms: u64,
    pub steps_executed: usize,
    pub agent_contributions: HashMap<String, usize>,
    pub findings: Vec<Finding>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
            Self::Info => "INFO",
        }
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}
