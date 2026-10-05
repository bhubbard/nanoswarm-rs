// ============================================================================
// tools/mod.rs — Built-in execution tools for NanoSwarm
// ============================================================================

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Tool definition and execution metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
}

/// Registry of tools available to agents in the swarm.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    allowed_root: Option<PathBuf>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { allowed_root: None }
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self {
            allowed_root: Some(root.into()),
        }
    }

    /// List standard built-in tools.
    pub fn descriptors(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor {
                name: "fs_read_file".into(),
                description: "Read the full contents of a file at the given path".into(),
                parameters_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Relative or absolute file path" }
                    },
                    "required": ["path"]
                }),
            },
            ToolDescriptor {
                name: "fs_write_file".into(),
                description: "Write content to a file at the given path (creates parent directories if needed)".into(),
                parameters_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Target file path" },
                        "content": { "type": "string", "description": "Full file content to write" }
                    },
                    "required": ["path", "content"]
                }),
            },
            ToolDescriptor {
                name: "fs_list_dir".into(),
                description: "List directory entries recursively up to max_depth".into(),
                parameters_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Directory path (default '.')" },
                        "max_depth": { "type": "integer", "description": "Maximum traversal depth (default 3)" }
                    }
                }),
            },
            ToolDescriptor {
                name: "fs_search".into(),
                description: "Search for a pattern or keyword across files in a directory".into(),
                parameters_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": { "type": "string", "description": "Substring or regex pattern to search for" },
                        "path": { "type": "string", "description": "Root directory to search (default '.')" },
                        "extension": { "type": "string", "description": "Optional file extension filter e.g. 'rs' or 'ts'" }
                    },
                    "required": ["pattern"]
                }),
            },
            ToolDescriptor {
                name: "shell_run".into(),
                description: "Execute a shell command with a safety timeout and capture its stdout/stderr".into(),
                parameters_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "Command string to execute via sh" },
                        "timeout_secs": { "type": "integer", "description": "Maximum seconds before termination (default 30)" }
                    },
                    "required": ["command"]
                }),
            },
            ToolDescriptor {
                name: "git_diff".into(),
                description: "Inspect unstaged and staged git changes in the repository".into(),
                parameters_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "cached": { "type": "boolean", "description": "If true, inspect staged changes (--cached)" }
                    }
                }),
            },
        ]
    }

    /// Dispatch a tool call by name with JSON arguments.
    pub async fn execute(&self, tool_name: &str, arguments: &serde_json::Value) -> Result<String> {
        match tool_name {
            "fs_read_file" => {
                let path_str = arguments
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                self.read_file(path_str).await
            }
            "fs_write_file" => {
                let path_str = arguments
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                let content = arguments
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'content' argument"))?;
                self.write_file(path_str, content).await
            }
            "fs_list_dir" => {
                let path_str = arguments
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                let max_depth = arguments
                    .get("max_depth")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(3) as usize;
                self.list_dir(path_str, max_depth).await
            }
            "fs_search" => {
                let pattern = arguments
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'pattern' argument"))?;
                let path_str = arguments
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                let ext = arguments.get("extension").and_then(|v| v.as_str());
                self.search_files(path_str, pattern, ext).await
            }
            "shell_run" => {
                let command = arguments
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'command' argument"))?;
                let timeout_secs = arguments
                    .get("timeout_secs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30);
                self.shell_run(command, timeout_secs).await
            }
            "git_diff" => {
                let cached = arguments
                    .get("cached")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                self.git_diff(cached).await
            }
            unknown => Err(anyhow!("Unknown tool '{}'", unknown)),
        }
    }

    fn resolve_path(&self, path_str: &str) -> Result<PathBuf> {
        let path = Path::new(path_str);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(ref root) = self.allowed_root {
            root.join(path)
        } else {
            path.to_path_buf()
        };

        if let Some(ref root) = self.allowed_root {
            if let (Ok(can_res), Ok(can_root)) = (resolved.canonicalize(), root.canonicalize()) {
                if !can_res.starts_with(&can_root) {
                    return Err(anyhow!(
                        "Access denied: path '{}' escapes allowed root '{}'",
                        path_str,
                        root.display()
                    ));
                }
            }
        }

        Ok(resolved)
    }

    async fn read_file(&self, path_str: &str) -> Result<String> {
        let path = self.resolve_path(path_str)?;
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", path.display()));
        }
        let content = tokio::fs::read_to_string(&path).await?;
        Ok(content)
    }

    async fn write_file(&self, path_str: &str, content: &str) -> Result<String> {
        let path = self.resolve_path(path_str)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, content).await?;
        Ok(format!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            path.display()
        ))
    }

    async fn list_dir(&self, root_str: &str, max_depth: usize) -> Result<String> {
        let root = self.resolve_path(root_str)?;
        let mut entries = Vec::new();
        let walker = walkdir::WalkDir::new(&root)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "target" && name != "node_modules"
            });

        for entry in walker.filter_map(std::result::Result::ok) {
            if entry.depth() == 0 {
                continue;
            }
            let path_display = entry.path().display().to_string();
            let file_type = if entry.file_type().is_dir() {
                "DIR"
            } else {
                "FILE"
            };
            entries.push(format!("[{}] {}", file_type, path_display));
        }
        Ok(entries.join("\n"))
    }

    async fn search_files(
        &self,
        root_str: &str,
        pattern: &str,
        ext_filter: Option<&str>,
    ) -> Result<String> {
        let root = self.resolve_path(root_str)?;
        let mut matches = Vec::new();
        let walker = walkdir::WalkDir::new(&root).into_iter().filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "target" && name != "node_modules"
        });

        for entry in walker.filter_map(std::result::Result::ok) {
            if entry.file_type().is_file() {
                if let Some(ext) = ext_filter {
                    if entry.path().extension().and_then(|s| s.to_str()) != Some(ext) {
                        continue;
                    }
                }
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    for (line_idx, line) in content.lines().enumerate() {
                        if line.contains(pattern) {
                            matches.push(format!(
                                "{}:{}: {}",
                                entry.path().display(),
                                line_idx + 1,
                                line.trim()
                            ));
                            if matches.len() >= 50 {
                                matches.push("... [truncated after 50 matches]".into());
                                return Ok(matches.join("\n"));
                            }
                        }
                    }
                }
            }
        }
        if matches.is_empty() {
            Ok(format!("No matches found for '{}'", pattern))
        } else {
            Ok(matches.join("\n"))
        }
    }

    async fn shell_run(&self, command: &str, timeout_secs: u64) -> Result<String> {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stdout_handle = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Missing stdout"))?;
        let mut stderr_handle = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("Missing stderr"))?;

        let timeout = tokio::time::Duration::from_secs(timeout_secs);
        let res = tokio::time::timeout(timeout, async {
            let status = child.wait().await?;
            let mut out_str = String::new();
            let mut err_str = String::new();
            let _ = stdout_handle.read_to_string(&mut out_str).await;
            let _ = stderr_handle.read_to_string(&mut err_str).await;
            Ok::<_, anyhow::Error>((status, out_str, err_str))
        })
        .await;

        match res {
            Ok(Ok((status, out_str, err_str))) => {
                let code = status.code().unwrap_or(-1);
                let mut combined = String::new();
                if !out_str.is_empty() {
                    combined.push_str(&out_str);
                }
                if !err_str.is_empty() {
                    if !combined.is_empty() {
                        combined.push('\n');
                    }
                    combined.push_str(&format!("[STDERR] {}", err_str));
                }
                if combined.is_empty() {
                    combined = format!("[Process exited with code {}]", code);
                }
                Ok(combined)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow!("Command timed out after {} seconds", timeout_secs)),
        }
    }

    async fn git_diff(&self, cached: bool) -> Result<String> {
        let mut cmd = Command::new("git");
        cmd.arg("diff");
        if cached {
            cmd.arg("--cached");
        }
        let output = cmd.output().await?;
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        if text.is_empty() {
            Ok("No changes detected in git diff".into())
        } else {
            Ok(text)
        }
    }
}
