### 1、a mini agent

tag: v0.0.1  
build a mini agent from scratch

---

## 环境要求

- 若**从源码编译**：需安装 [Rust](https://www.rust-lang.org/tools/install)，且版本满足 `Cargo.toml` 中的 `rust-version`（当前为 **1.92+**）。
- 若使用 **GitHub Releases** 预编译包：无需安装 Rust，将对应平台的二进制放到 `PATH` 即可。

## 安装

### 从源码构建（各系统相同）

在项目根目录执行：

```bash
cargo build --release
```

可执行文件路径：

| 系统 | 路径 |
|------|------|
| Windows | `target\release\agentlite.exe` |
| Ubuntu / macOS | `target/release/agentlite` |

可将该文件复制到任意目录并加入 `PATH`，或直接用 `cargo run --release --` 运行。

### Windows

1. 安装 Rust：在 [rustup.rs](https://rustup.rs/) 下载并安装，用 **x64 Native Tools** 或已带链接环境的终端执行构建。
2. 进入项目目录后执行 `cargo build --release`。
3. 将 `target\release\agentlite.exe` 放到希望使用的目录（可选：加入系统或用户 `PATH`）。

### Ubuntu（及其他 Linux）

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev   # 若缺依赖再装
cd /path/to/agentlite
cargo build --release
sudo cp target/release/agentlite /usr/local/bin/   # 可选，方便全局调用
```

### macOS

```bash
xcode-select --install   # 若尚未安装命令行工具
cd /path/to/agentlite
cargo build --release
cp target/release/agentlite /usr/local/bin/   # 可选
```

---

## 配置

程序通过环境变量连接 DeepSeek（OpenAI 兼容接口）：

| 变量 | 是否必填 | 说明 |
|------|----------|------|
| `DEEPSEEK_API_KEY` | **必填** | DeepSeek API 密钥。未设置时程序会退出并提示错误。 |
| `DEEPSEEK_BASE_URL` | 可选 | API 地址，默认 `https://api.deepseek.com`。 |

### Windows（PowerShell，当前会话）

```powershell
$env:DEEPSEEK_API_KEY = "你的密钥"
# 可选
$env:DEEPSEEK_BASE_URL = "https://api.deepseek.com"
```

### Windows（CMD，当前会话）

```cmd
set DEEPSEEK_API_KEY=你的密钥
set DEEPSEEK_BASE_URL=https://api.deepseek.com
```

### Ubuntu / macOS（Bash/Zsh，当前会话）

```bash
export DEEPSEEK_API_KEY="你的密钥"
# 可选
export DEEPSEEK_BASE_URL="https://api.deepseek.com"
```

将 `export` 行写入 `~/.bashrc`、`~/.zshrc` 等可在新终端里长期生效（改完后执行 `source ~/.bashrc` 或重新打开终端）。

---

## 使用方法

在已配置好环境变量的终端中：

```bash
agentlite -p "你的问题或任务描述"
```

或长参数：

```bash
agentlite --prompt "你的问题或任务描述"
```

从源码目录直接运行（不拷贝二进制）：

```bash
cargo run --release -- -p "你的问题或任务描述"
```

查看帮助：

```bash
agentlite --help
```

程序会把模型回复打印到标准输出。
