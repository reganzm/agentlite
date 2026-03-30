use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{NaiveDate, SecondsFormat, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

const PREVIEW_MAX_CHARS: usize = 512;

/// New unique id for one agent task / invocation chain (correlate audit lines).
pub fn new_trace_id() -> String {
    Uuid::new_v4().to_string()
}

/// Parent directory of the running executable (e.g. where `agentlite.exe` lives).
pub fn executable_dir() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "current_exe has no parent",
            )
        })
}

fn default_logs_dir() -> PathBuf {
    executable_dir()
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("logs")
}

/// Daily-rotated files under `dir`: `tool-audit-YYYY-MM-DD.log` (UTC).
/// Or a single fixed file when using [`AuditMode::Fixed`].
#[derive(Debug, Clone)]
enum AuditMode {
    Daily { dir: PathBuf },
    Fixed { path: PathBuf },
}

struct AuditState {
    open_date: NaiveDate,
    path: PathBuf,
    file: File,
}

impl AuditState {
    fn open_initial(mode: &AuditMode) -> std::io::Result<Self> {
        match mode {
            AuditMode::Daily { dir } => {
                std::fs::create_dir_all(dir)?;
                let today = Utc::now().date_naive();
                let path = dir.join(daily_filename(today));
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)?;
                Ok(Self {
                    open_date: today,
                    path,
                    file,
                })
            }
            AuditMode::Fixed { path } => {
                if let Some(p) = path.parent() {
                    if !p.as_os_str().is_empty() {
                        std::fs::create_dir_all(p)?;
                    }
                }
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?;
                let today = Utc::now().date_naive();
                Ok(Self {
                    open_date: today,
                    path: path.clone(),
                    file,
                })
            }
        }
    }

    /// For [`AuditMode::Daily`], switch file when UTC calendar day changes.
    fn ensure_current_file(&mut self, mode: &AuditMode) -> std::io::Result<()> {
        let AuditMode::Daily { dir } = mode else {
            return Ok(());
        };
        let today = Utc::now().date_naive();
        if self.open_date == today {
            return Ok(());
        }
        std::fs::create_dir_all(dir)?;
        let path = dir.join(daily_filename(today));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        *self = Self {
            open_date: today,
            path,
            file,
        };
        Ok(())
    }
}

fn daily_filename(date: NaiveDate) -> String {
    format!("tool-audit-{}.log", date)
}

fn extension_is_log(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("log"))
}

fn parse_env_path(p: &Path) -> std::io::Result<AuditMode> {
    if p.as_os_str().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "empty AGENTLITE_TOOL_LOG path",
        ));
    }
    if p.exists() && p.is_dir() {
        return Ok(AuditMode::Daily {
            dir: p.to_path_buf(),
        });
    }
    if p.exists() && p.is_file() {
        return Ok(AuditMode::Fixed {
            path: p.to_path_buf(),
        });
    }
    if extension_is_log(p) {
        return Ok(AuditMode::Fixed {
            path: p.to_path_buf(),
        });
    }
    Ok(AuditMode::Daily {
        dir: p.to_path_buf(),
    })
}

/// Append-only JSON Lines audit log for tool invocations (one record per line).
pub struct ToolAuditLog {
    mode: AuditMode,
    state: Mutex<AuditState>,
}

/// One tool call audit line (serializable to JSON).
#[derive(Debug, Serialize)]
pub struct ToolAuditRecord {
    /// User/chat session (product: one id per conversation; CLI placeholder: see `session` module).
    pub session_id: String,
    /// Correlates all tool calls within one user task / agent run.
    pub trace_id: String,
    /// OpenAI `tool_calls[].id` for this step, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// RFC3339 with millisecond precision (UTC).
    pub timestamp: String,
    /// UTC epoch milliseconds at invocation start (same instant as `timestamp`).
    pub invoked_at_ms: i64,
    /// Tool name as seen by the model (exposed name).
    pub tool: String,
    /// Arguments object from the model.
    pub arguments: Value,
    /// `in_process` or `mcp`.
    pub backend: String,
    /// When `backend` is MCP, the remote server's tool name if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server_tool: Option<String>,
    /// Wall time for this invocation (single backend try).
    pub duration_ms: f64,
    /// `success`, `tool_returned_error`, or `unknown_tool`.
    pub status: &'static str,
    pub result_length: usize,
    pub result_preview: String,
}

impl ToolAuditLog {
    /// Configure from `AGENTLITE_TOOL_LOG`:
    /// - unset → `{executable_dir}/logs/tool-audit-YYYY-MM-DD.log` (UTC, new file each day)
    /// - `disabled` / `0` / empty → no logging (`None`)
    /// - existing directory or non-existent path → daily files under that directory
    /// - path ending in `.log` (and not an existing directory) → single file, no rotation
    pub fn from_env() -> std::io::Result<Option<Arc<Self>>> {
        let mode = match std::env::var("AGENTLITE_TOOL_LOG") {
            Ok(s) => {
                let t = s.trim();
                if t.is_empty() || t.eq_ignore_ascii_case("disabled") || t == "0" {
                    return Ok(None);
                }
                parse_env_path(Path::new(t))?
            }
            Err(_) => AuditMode::Daily {
                dir: default_logs_dir(),
            },
        };
        let state = AuditState::open_initial(&mode)?;
        Ok(Some(Arc::new(Self {
            mode,
            state: Mutex::new(state),
        })))
    }

    /// Open a daily-rotating log under `logs_dir`, or a fixed file if `path` is a `.log` file.
    #[allow(dead_code)] // Public hook for embedders; default path uses [`from_env`].
    pub fn open_path(path: impl AsRef<Path>) -> std::io::Result<Arc<Self>> {
        let p = path.as_ref();
        let mode = if p.exists() && p.is_dir() {
            AuditMode::Daily {
                dir: p.to_path_buf(),
            }
        } else if p.exists() && p.is_file() {
            AuditMode::Fixed {
                path: p.to_path_buf(),
            }
        } else if extension_is_log(p) {
            AuditMode::Fixed {
                path: p.to_path_buf(),
            }
        } else {
            AuditMode::Daily {
                dir: p.to_path_buf(),
            }
        };
        let state = AuditState::open_initial(&mode)?;
        Ok(Arc::new(Self {
            mode,
            state: Mutex::new(state),
        }))
    }

    pub fn record(&self, record: &ToolAuditRecord) {
        let line = match serde_json::to_string(record) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("agentlite tool audit: serialize failed: {}", e);
                return;
            }
        };
        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("agentlite tool audit: lock failed: {}", e);
                return;
            }
        };
        if let Err(e) = state.ensure_current_file(&self.mode) {
            eprintln!("agentlite tool audit: rotate/open log: {}", e);
            return;
        }
        let path_for_err = state.path.display().to_string();
        if let Err(e) = writeln!(state.file, "{}", line) {
            eprintln!(
                "agentlite tool audit: write {} failed: {}",
                path_for_err, e
            );
            return;
        }
        let _ = state.file.flush();
    }
}

pub fn utc_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// UTC epoch milliseconds when the tool invocation begins (start of `ToolCatalog::execute`).
pub fn invoked_at_timestamp_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub fn classify_status(result: &str) -> &'static str {
    if result.starts_with("Error:") || result.starts_with("MCP error:") {
        "tool_returned_error"
    } else {
        "success"
    }
}

pub fn truncate_preview(s: &str, max: usize) -> String {
    let mut it = s.chars();
    let prefix: String = it.by_ref().take(max).collect();
    if it.next().is_some() {
        format!("{}…", prefix)
    } else {
        prefix
    }
}

pub fn preview_result(result: &str) -> String {
    truncate_preview(result, PREVIEW_MAX_CHARS)
}
