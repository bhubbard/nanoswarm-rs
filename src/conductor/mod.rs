// ============================================================================
// conductor/mod.rs — Swarm Conductor & Orchestrator
// ============================================================================

pub mod router;

pub use router::SwarmRouter;

use anyhow::Result;
use apfel::backend::BackendEngine;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::agent::Agent;
use crate::tools::ToolRegistry;
use crate::types::{AgentSpec, Finding, Severity, SwarmEvent, SwarmReport, Task};

/// Orchestrates a swarm of autonomous agents to achieve a high-level goal.
pub struct Conductor {
    pub name: String,
    pub agents: Vec<AgentSpec>,
    pub tools: ToolRegistry,
    pub engine: Arc<dyn BackendEngine>,
    pub max_turns: usize,
}

impl Conductor {
    pub fn new(
        name: impl Into<String>,
        engine: Arc<dyn BackendEngine>,
        tools: ToolRegistry,
    ) -> Self {
        Self {
            name: name.into(),
            agents: Vec::new(),
            tools,
            engine,
            max_turns: 8,
        }
    }

    pub fn register_agent(&mut self, spec: AgentSpec) {
        self.agents.push(spec);
    }

    pub fn with_agent(mut self, spec: AgentSpec) -> Self {
        self.register_agent(spec);
        self
    }

    /// Executes a collaborative swarm run targeting a goal.
    pub async fn run(
        &self,
        task: &Task,
        event_tx: Option<mpsc::Sender<SwarmEvent>>,
    ) -> Result<SwarmReport> {
        let start_time = Instant::now();

        if let Some(ref tx) = event_tx {
            let _ = tx
                .send(SwarmEvent::SwarmStarted {
                    swarm_name: self.name.clone(),
                    goal: task.goal.clone(),
                })
                .await;
        }

        let mut contributions: HashMap<String, usize> = HashMap::new();
        let mut steps_executed = 0;
        let mut current_context = task.context.clone();
        let mut findings = Vec::new();
        let mut final_summary = String::new();

        for turn in 0..self.max_turns {
            steps_executed += 1;

            // Step 1: Use Zev microsecond router to select the ideal agent for this turn
            let subtask_prompt = format!(
                "Current Goal: {}. Progress Context:\n{}",
                task.goal, current_context
            );
            let selected_spec = SwarmRouter::select_best_agent(&subtask_prompt, &self.agents)
                .cloned()
                .unwrap_or_else(|| self.agents[0].clone());

            *contributions.entry(selected_spec.id.0.clone()).or_insert(0) += 1;

            if let Some(ref tx) = event_tx {
                let _ = tx
                    .send(SwarmEvent::AgentAssigned {
                        agent_id: selected_spec.id.clone(),
                        role: selected_spec.role.clone(),
                        subtask: format!("Turn {}/{}", turn + 1, self.max_turns),
                    })
                    .await;
            }

            // Step 2: Instantiate agent worker and execute with local tools
            let mut worker = Agent::new(
                selected_spec.clone(),
                self.engine.clone(),
                self.tools.clone(),
            );

            let prompt = format!(
                "Task Goal: {}\nOverall Context:\n{}\n\nPlease advance this goal. Use available tools as needed. If you discover actionable findings or conclusions, state them clearly.",
                task.goal, current_context
            );

            let turn_output = worker.execute(&prompt, 4, event_tx.as_ref()).await?;

            // Append turn output to accumulated context
            current_context.push_str(&format!(
                "\n--- [{}] {}\n{}\n",
                selected_spec.role.as_str(),
                selected_spec.id.0,
                turn_output
            ));

            final_summary = turn_output.clone();

            // Extract any findings if formatted
            self.extract_findings(&turn_output, &mut findings);

            // Step 3: Check completion guardrail using Zev
            let is_complete = SwarmRouter::is_task_complete(&turn_output, &task.goal);
            if is_complete && turn > 0 {
                if let Some(ref tx) = event_tx {
                    let _ = tx
                        .send(SwarmEvent::TaskCompleted {
                            agent_id: selected_spec.id.clone(),
                            summary: format!("Task verified complete after {} turns", turn + 1),
                        })
                        .await;
                }
                break;
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if let Some(ref tx) = event_tx {
            let _ = tx
                .send(SwarmEvent::SwarmFinished {
                    duration_ms,
                    total_steps: steps_executed,
                })
                .await;
        }

        Ok(SwarmReport {
            goal: task.goal.clone(),
            swarm_name: self.name.clone(),
            duration_ms,
            steps_executed,
            agent_contributions: contributions,
            findings,
            summary: final_summary,
        })
    }

    /// Parallel fan-out execution across all agents simultaneously.
    pub async fn run_parallel(
        &self,
        task: &Task,
        event_tx: Option<mpsc::Sender<SwarmEvent>>,
    ) -> Result<SwarmReport> {
        let start_time = Instant::now();

        if let Some(ref tx) = event_tx {
            let _ = tx
                .send(SwarmEvent::SwarmStarted {
                    swarm_name: self.name.clone(),
                    goal: task.goal.clone(),
                })
                .await;
        }

        let mut handles = Vec::new();
        let mut contributions: HashMap<String, usize> = HashMap::new();

        for spec in &self.agents {
            *contributions.entry(spec.id.0.clone()).or_insert(0) += 1;
            let mut worker = Agent::new(spec.clone(), self.engine.clone(), self.tools.clone());
            let goal_clone = task.goal.clone();
            let context_clone = task.context.clone();
            let tx_clone = event_tx.clone();

            let handle = tokio::spawn(async move {
                if let Some(ref tx) = tx_clone {
                    let _ = tx
                        .send(SwarmEvent::AgentAssigned {
                            agent_id: worker.id().clone(),
                            role: worker.role().clone(),
                            subtask: "Parallel analysis".into(),
                        })
                        .await;
                }

                let prompt = format!(
                    "Goal: {}\nContext:\n{}\n\nPerform your specialized analysis as {}.",
                    goal_clone,
                    context_clone,
                    worker.role().as_str()
                );

                let output = worker.execute(&prompt, 3, tx_clone.as_ref()).await?;
                Ok::<_, anyhow::Error>((worker.id().clone(), worker.role().clone(), output))
            });

            handles.push(handle);
        }

        let mut findings = Vec::new();
        let mut outputs = Vec::new();

        for h in handles {
            if let Ok(Ok((id, role, output))) = h.await {
                self.extract_findings(&output, &mut findings);
                outputs.push(format!("### [{}] {}\n\n{}", role.as_str(), id, output));
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let total_steps = self.agents.len();

        if let Some(ref tx) = event_tx {
            let _ = tx
                .send(SwarmEvent::SwarmFinished {
                    duration_ms,
                    total_steps,
                })
                .await;
        }

        Ok(SwarmReport {
            goal: task.goal.clone(),
            swarm_name: self.name.clone(),
            duration_ms,
            steps_executed: total_steps,
            agent_contributions: contributions,
            findings,
            summary: outputs.join("\n\n"),
        })
    }

    fn extract_findings(&self, text: &str, findings: &mut Vec<Finding>) {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("[CRITICAL]") || trimmed.starts_with("CRITICAL:") {
                findings.push(Finding {
                    severity: Severity::Critical,
                    title: trimmed.to_string(),
                    description: trimmed.to_string(),
                    file_path: None,
                    line_number: None,
                    recommendation: None,
                });
            } else if trimmed.starts_with("[HIGH]") || trimmed.starts_with("HIGH:") {
                findings.push(Finding {
                    severity: Severity::High,
                    title: trimmed.to_string(),
                    description: trimmed.to_string(),
                    file_path: None,
                    line_number: None,
                    recommendation: None,
                });
            } else if trimmed.starts_with("[MEDIUM]") || trimmed.starts_with("MEDIUM:") {
                findings.push(Finding {
                    severity: Severity::Medium,
                    title: trimmed.to_string(),
                    description: trimmed.to_string(),
                    file_path: None,
                    line_number: None,
                    recommendation: None,
                });
            }
        }
    }
}
