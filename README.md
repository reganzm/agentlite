### 1. A mini agent

tag: v0.0.1  
Build a mini agent from scratch.

---

## Requirements

- **Building from source:** Install [Rust](https://www.rust-lang.org/tools/install) and use a toolchain that satisfies `rust-version` in `Cargo.toml` (currently **1.92+**).
- **Prebuilt binaries from GitHub Releases:** No Rust needed; put the binary for your platform on your `PATH`.

## Installation

### Build from source (all platforms)

From the repository root:

```bash
cargo build --release
```

Output locations:

| OS | Path |
|----|------|
| Windows | `target\release\agentlite.exe` |
| Ubuntu / macOS | `target/release/agentlite` |

Copy the binary wherever you like and add it to `PATH`, or run with `cargo run --release --` (see below).

### Windows

1. Install Rust from [rustup.rs](https://rustup.rs/). Build from a shell that has the MSVC linker available (e.g. **x64 Native Tools** or a normal Developer environment).
2. In the project directory, run `cargo build --release`.
3. Optionally copy `target\release\agentlite.exe` to a folder on your user or system `PATH`.

### Ubuntu (and other Linux)

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev   # install if anything is missing
cd /path/to/agentlite
cargo build --release
sudo cp target/release/agentlite /usr/local/bin/   # optional, for global use
```

### macOS

```bash
xcode-select --install   # if Command Line Tools are not installed yet
cd /path/to/agentlite
cargo build --release
cp target/release/agentlite /usr/local/bin/   # optional
```

---

## Configuration

The program talks to DeepSeek (OpenAI-compatible API) via environment variables:

| Variable | Required | Description |
|----------|----------|-------------|
| `DEEPSEEK_API_KEY` | **Yes** | DeepSeek API key. The process exits with an error if unset. |
| `DEEPSEEK_BASE_URL` | No | API base URL; defaults to `https://api.deepseek.com`. |
| `AGENTLITE_TOOL_LOG` | No | **Tool-call audit log** as **JSON Lines** (one JSON object per line). If unset: writes under **`{executable directory}/logs/tool-audit-YYYY-MM-DD.log`** (**UTC daily rotation**; a new file when the UTC day changes). Set to `disabled` or `0` to disable. If set to a path: an existing or new directory is treated as the **log directory** (daily files); an existing file or a path ending in `.log` that is not a directory is a **single fixed file** (no daily rotation). |
| `AGENTLITE_MCP_CONFIG` | No | Path to a JSON file listing MCP servers. See `mcp.config.example.json`. **Local stdio:** set `command` (+ optional `args`, `env`). **Remote http(s):** set `url` to an `http://` or `https://` MCP *streamable HTTP* endpoint; optional `headers` (string map) and `bearer_token` (sent as Authorization; value is the raw token without a `Bearer ` prefix, per rmcp). Do not set `command` when using `url`. |
| `AGENTLITE_SESSION_ID` | No | **Session id** (placeholder). Target behavior: like a browser session (login through logout) or **one `session_id` per opened chat**, provided by the host. Current CLI: if unset, a **new random UUID** is generated on each process start (no file); if set, that value is used (e.g. integration testing). |

Session vs audit (target model): **one `session_id` → multiple tasks within the same conversation**; **one `trace_id` → multiple `tool_call`s in one `run`**. In the current CLI, each launch usually gets a new placeholder `session_id`.

Each audit line includes: `session_id`, `trace_id` (unchanged for one `run`), `tool_call_id` (model `tool_calls[].id`), `timestamp` (RFC3339), `invoked_at_ms` (UTC epoch ms at call start, aligned with `timestamp`), `tool`, `arguments`, `backend`, `mcp_server_tool`, `duration_ms` (execution time in ms), `status`, `result_length`, `result_preview`. You can aggregate by `session_id`, then by `trace_id` for a single task’s tool chain.

### Windows (PowerShell, current session)

```powershell
$env:DEEPSEEK_API_KEY = "your-api-key"
# optional
$env:DEEPSEEK_BASE_URL = "https://api.deepseek.com"
```

### Windows (CMD, current session)

```cmd
set DEEPSEEK_API_KEY=your-api-key
set DEEPSEEK_BASE_URL=https://api.deepseek.com
```

### Ubuntu / macOS (Bash/Zsh, current session)

```bash
export DEEPSEEK_API_KEY="your-api-key"
# optional
export DEEPSEEK_BASE_URL="https://api.deepseek.com"
```

Add the `export` lines to `~/.bashrc`, `~/.zshrc`, etc. for persistence in new shells (then `source ~/.bashrc` or open a new terminal).

---

## Usage

With the environment variables set in your shell:

```bash
agentlite -p "your question or task"
```

Long form:

```bash
agentlite --prompt "your question or task"
```

Run from the repo without installing the binary:

```bash
cargo run --release -- -p "your question or task"
```

Help:

```bash
agentlite --help
```

The model response is printed to standard output.
