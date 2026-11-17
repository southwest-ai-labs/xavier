# Xavier — Windows Installation

## Requirements

- Windows 10/11 (64-bit)
- Optional: Ollama (for offline local embeddings) → https://ollama.com

---

## 1. Official Graphical Installers (Recommended)

Prebuilt official installers are published with every release on [GitHub Releases](https://github.com/southwest-ai-labs/xavier/releases/latest):

| Installer | Format | Best For | Description |
|---|---|---|---|
| **[XavierSetup.exe](https://github.com/southwest-ai-labs/xavier/releases/download/v0.2.19/XavierSetup.exe)** | `.exe` (Inno Setup) | Desktop users & Developers | Full graphical setup wizard. Installs `xavier.exe`, `xavier-tui.exe`, and Panel Web UI assets. Automatically creates Start Menu shortcuts and configures PATH. |
| **[XavierInstaller.msi](https://github.com/southwest-ai-labs/xavier/releases/download/v0.2.19/XavierInstaller.msi)** | `.msi` (WiX) | Enterprise & Automated CI | Machine-wide MSI installer with support for silent GPO/MDM deployment (`msiexec /i XavierInstaller.msi /qn`). |
| **[xavier-v0.2.19-x86_64-pc-windows-msvc.zip](https://github.com/southwest-ai-labs/xavier/releases/download/v0.2.19/xavier-v0.2.19-x86_64-pc-windows-msvc.zip)** | `.zip` | Portable / Ad-hoc | Unpack and run `xavier.exe` without installation. |

### How Xavier Runs in Windows (CLI vs Web Panel)

- **Starting the Server & Web Panel**:
  Launch `Xavier` from the Start Menu or run in terminal:
  ```powershell
  xavier http 8006
  ```
  This starts the core memory daemon and serves the visual administration dashboard at:
  👉 **`http://localhost:8006`**
- **1-Click Web Panel Shortcut**:
  The installer automatically creates a **`Xavier Web Panel`** shortcut on your Desktop and Start Menu that opens directly in your default browser with official branding and zero console popup window.
- **Local Offline Embeddings (Zero API Keys)**:
  Xavier automatically integrates with local [Ollama](https://ollama.com) via `http://localhost:11434` using `nomic-embed-text` (768 dimensions, 8,192 token window). Once Ollama is running (`ollama serve`), Xavier vectorizes all memories and agent contexts 100% locally with zero cloud API keys or monthly costs.
- **Terminal Monitor (TUI)**:
  Launch `Xavier TUI` from the Start Menu or run `xavier-tui` in PowerShell for live CPU/RAM and memory metrics.
- **Headless Server (No UI)**:
  If you are embedding Xavier into automated pipelines and do not need the Web UI:
  ```powershell
  xavier http --no-ui
  ```

---

## 2. Quick Command-Line Install (PowerShell Script)

You can install Xavier with a single PowerShell command without installing Rust or compiling from source. The installer automatically downloads prebuilt binaries and Web UI assets from GitHub Releases:

```powershell
irm https://raw.githubusercontent.com/southwest-ai-labs/xavier/main/scripts/install.ps1 | iex
```

### Script Options & Silent Installation

When running `install.ps1` locally or non-interactively:

```powershell
# Automatically download latest prebuilt release binary and UI assets
.\scripts\install.ps1 -DownloadLatest

# Silent unattended installation with logging
.\scripts\install.ps1 -DownloadLatest -Silent -LogPath "$env:TEMP\xavier-install.log" -StartService

# Exit codes:
#  0 = Success
#  1 = General error
#  2 = SHA-256 integrity mismatch
#  3 = Missing xavier.exe in downloaded asset
```

## Building & Installing from Source

If you prefer building from source:

```powershell
# 1. Build binary (from project root)
cargo build --release --bin xavier -j 1

# 2. Build Web UI (optional, for web dashboard)
cd panel-ui
pnpm install
pnpm build
cd ..

# 3. Run installer
.\scripts\install.ps1
```

## What the Installer Does

This installer:
- Deploys `xavier.exe` to `%LOCALAPPDATA%\Xavier\bin\`
- Copies Web UI assets (`panel-ui/build`) to `%LOCALAPPDATA%\Xavier\panel-ui\build`
- Adds `%LOCALAPPDATA%\Xavier\bin\` to the User `PATH`
- Generates a random secure `XAVIER_TOKEN`
- Creates `%LOCALAPPDATA%\Xavier\data\` for local SQLite database storage
- Copies `.env.example` to `%LOCALAPPDATA%\Xavier\config\.env`

> **Restart your terminal** after installation for the `PATH` environment variable changes to take effect.

## Usage

```powershell
# Help
xavier --help

# Start server (web dashboard available at http://localhost:8006)
xavier http

# Or alias
xavier serve

# Check doctor and status
xavier doctor
xavier status
```

## Configuration

Edit `%LOCALAPPDATA%\Xavier\config\.env` with your values:

| Variable | Description | Default |
|---|---|---|
| `XAVIER_TOKEN` | API token (required) | (generated) |
| `XAVIER_PORT` | Server port | 8006 |
| `XAVIER_MEMORY_BACKEND` | Backend: `vec` (SQLite) or `surreal` | `vec` |
| `XAVIER_EMBEDDING_URL` | Embeddings URL (Ollama) | `http://localhost:11434/v1` |
| `XAVIER_EMBEDDING_MODEL` | Embeddings model | `nomic-embed-text` |
| `XAVIER_MODEL_PROVIDER` | LLM provider | `local` |
| `RUST_LOG` | Logging level | `info` |

## Docker (Containerized Alternative)

If you prefer running in Docker on Windows via WSL2:

```powershell
docker run -d --name xavier -p 8006:8006 -p 8100:8100 -v xavier_data:/data ghcr.io/southwest-ai-labs/xavier:latest
```

## Uninstall

```powershell
# Remove from PATH
$path = [Environment]::GetEnvironmentVariable("PATH", "User")
$path = ($path.Split(';') | Where-Object { $_ -ne "$env:LOCALAPPDATA\Xavier\bin" }) -join ';'
[Environment]::SetEnvironmentVariable("PATH", $path, "User")

# Delete directory
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\Xavier"
```
