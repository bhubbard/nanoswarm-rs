// ============================================================================
// agent/worker.rs — Autonomous Agent worker runtime
// ============================================================================

use anyhow::{anyhow, Result};
use apfel::backend::{BackendEngine, GenerateRequest};
use apfel::core::models::OpenAIMessage;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::tools::ToolRegistry;
use crate::types::{AgentId, AgentRole, AgentSpec, SwarmEvent};

/// An active autonomous agent instance.
pub struct Agent {
    pub spec: AgentSpec,
    pub engine: Arc<dyn BackendEngine>,
    pub tools: ToolRegistry,
    pub history: Vec<OpenAIMessage>,
}

impl Agent {
    pub fn new(spec: AgentSpec, engine: Arc<dyn BackendEngine>, tools: ToolRegistry) -> Self {
        let history = vec![OpenAIMessage::system(&spec.system_prompt)];
        Self {
            spec,
            engine,
            tools,
            history,
        }
    }

    pub fn id(&self) -> &AgentId {
        &self.spec.id
    }

    pub fn role(&self) -> &AgentRole {
        &self.spec.role
    }

    /// Single execution turn: appends user prompt, generates completion, executes any tool calls,
    /// and loops up to `max_steps` until a terminal response is generated.
    pub async fn execute(
        &mut self,
        prompt: &str,
        max_steps: usize,
        event_tx: Option<&mpsc::Sender<SwarmEvent>>,
    ) -> Result<String> {
        self.history.push(OpenAIMessage::user(prompt));

        let mut step = 0;
        let mut final_response = String::new();

        while step < max_steps {
            step += 1;

            if let Some(tx) = event_tx {
                let _ = tx
                    .send(SwarmEvent::AgentThinking {
                        agent_id: self.spec.id.clone(),
                        preview: format!("Step {}/{}", step, max_steps),
                    })
                    .await;
            }

            let req = GenerateRequest {
                prompt: String::new(),
                system_prompt: Some(self.spec.system_prompt.clone()),
                messages: Some(self.history.clone()),
                temperature: self.spec.temperature,
                top_p: Some(0.9),
                max_tokens: self.spec.max_tokens,
                permissive: true,
                seed: None,
                use_case: None,
            };

            let resp = self
                .engine
                .generate(&req)
                .map_err(|e| anyhow!("Inference failed: {}", e))?;

            let content = resp.content;
            self.history.push(OpenAIMessage::assistant(&content));

            // Check if model emitted a tool call JSON block
            if let Some(tool_call) = self.parse_tool_call(&content) {
                if let Some(tx) = event_tx {
                    let _ = tx
                        .send(SwarmEvent::ToolExecuted {
                            agent_id: self.spec.id.clone(),
                            tool: tool_call.name.clone(),
                            input: tool_call.arguments.to_string(),
                            output: "[executing...]".into(),
                        })
                        .await;
                }

                let tool_result = match self
                    .tools
                    .execute(&tool_call.name, &tool_call.arguments)
                    .await
                {
                    Ok(res) => res,
                    Err(err) => format!("Error executing tool '{}': {}", tool_call.name, err),
                };

                let tool_feedback = format!("TOOL RESULT [{}]:\n{}", tool_call.name, tool_result);
                self.history.push(OpenAIMessage::user(&tool_feedback));
                continue;
            }

            // No tool call requested — terminal output generated
            final_response = content;
            break;
        }

        if let Some(tx) = event_tx {
            let _ = tx
                .send(SwarmEvent::AgentOutput {
                    agent_id: self.spec.id.clone(),
                    content: final_response.clone(),
                })
                .await;
        }

        Ok(final_response)
    }

    fn parse_tool_call(&self, text: &str) -> Option<ParsedToolCall> {
        // Look for ```json { "tool": "...", "arguments": { ... } } ``` or raw JSON
        if let Some(start_idx) = text.find("```json") {
            let rest = &text[start_idx + 7..];
            if let Some(end_idx) = rest.find("```") {
                let json_slice = rest[..end_idx].trim();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_slice) {
                    if let Some(call) = Self::extract_tool_from_val(&val) {
                        return Some(call);
                    }
                }
            }
        }

        // Try parsing entire text or braces if small
        if let Some(brace_start) = text.find('{') {
            if let Some(brace_end) = text.rfind('}') {
                if brace_end > brace_start {
                    let slice = &text[brace_start..=brace_end];
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(slice) {
                        if let Some(call) = Self::extract_tool_from_val(&val) {
                            return Some(call);
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_tool_from_val(val: &serde_json::Value) -> Option<ParsedToolCall> {
        let tool_name = val.get("tool").and_then(|v| v.as_str())?;
        let arguments = val
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        Some(ParsedToolCall {
            name: tool_name.to_string(),
            arguments,
        })
    }
}

#[derive(Debug, Clone)]
struct ParsedToolCall {
    name: String,
    arguments: serde_json::Value,
}
