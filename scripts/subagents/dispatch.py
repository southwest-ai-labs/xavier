#!/usr/bin/env python3
"""
dispatch.py — Orquestador unico de subagents para Xavier como cerebro.

Subagents soportados:
  1. AGI CLIs (sincrono): codex / opencode / gemini / claude / qwen
     - Lanzados via subprocess con wiring MCP (config en mcp/GENERATED/)
     - Bloquean hasta terminar; capturan stdout.
  2. Google Antigravity & agy CLI:
     - Runner canonico activo para Xavier waves y tareas locales.
     - Nota: Jules service ha sido desmantelado / decommissioned.

Flujo comun (Xavier como cerebro):
  RECALL  -> mem_search en Xavier (path_prefix del namespace) para contexto previo
  DISPATCH-> lanza el subagent con el contexto inyectado
  PERSIST -> create_memory con el resultado/decision

Uso:
  python dispatch.py agi --cli opencode --task "..." --project xavier
  python dispatch.py jules                   # [DEPRECATED] Jules decommissioned, usar Antigravity/agy
  python dispatch.py jules-status            # [DEPRECATED]

Requisitos:
  - Xavier HTTP corriendo en :8006 (modo cloud embedding)
  - configs MCP generados: python generate_mcp_configs.py
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
MCP_DIR = REPO_ROOT / "scripts" / "subagents" / "mcp" / "GENERATED"
REPORTS_DIR = REPO_ROOT / "scripts" / "subagents" / "reports"
REPORTS_DIR.mkdir(parents=True, exist_ok=True)

BASE = os.environ.get("XAVIER_BASE_URL", "http://localhost:8006")
DEFAULT_REPO = os.environ.get("XAVIER_GH_REPO", "iberi22/xavier")


# ---------------------------------------------------------------------------
# Xavier HTTP helpers
# ---------------------------------------------------------------------------
def _token() -> str:
    for line in (REPO_ROOT / ".env").read_text(encoding="utf-8").splitlines():
        if line.startswith("XAVIER_TOKEN=") and not line.startswith("#"):
            return line.split("=", 1)[1].strip()
    sys.exit("ERROR: XAVIER_TOKEN no encontrado en .env")


def _http(path: str, body: dict | None, method: str = "POST"):
    token = _token()
    req = urllib.request.Request(f"{BASE}{path}", method=method)
    req.add_header("Content-Type", "application/json")
    req.add_header("X-Xavier-Token", token)
    data = json.dumps(body).encode() if body else None
    try:
        with urllib.request.urlopen(req, data=data, timeout=60) as r:
            return json.loads(r.read().decode())
    except Exception as e:
        return {"_error": str(e)}


def recall(query: str, project: str | None, limit: int = 5) -> str:
    """Busca contexto previo en Xavier. Devuelve un bloque de texto para inyectar.

    Busca primero en el namespace del proyecto (path_prefix dispatch/{project}/), y si no
    hay resultados, hace una segunda busqueda global por project (sin path_prefix) para
    encontrar memorias heredadas de otros orígenes (locomo-eval, manual, etc.).
    """
    proj = project or "global"
    # 1er intento: namespace de dispatch del proyecto
    r = _http("/memory/search", {
        "query": query, "limit": limit,
        "filters": {"path_prefix": f"dispatch/{proj}/"},
    })
    results = r.get("results", []) if isinstance(r, dict) else []
    source = f"dispatch/{proj}/"
    # 2o intento: si vacio, buscar por project global (memorias de cualquier origen)
    if not results and project:
        r2 = _http("/memory/search", {
            "query": query, "limit": limit,
            "filters": {"project": project},
        })
        results = r2.get("results", []) if isinstance(r2, dict) else []
        source = f"project={project} (global)"
    if not results:
        return "(sin memoria previa en Xavier para este tema)"
    lines = [f"Memoria recuperada de Xavier ({len(results)} resultados, fuente={source}):"]
    for res in results[:limit]:
        c = (res.get("content") or "")[:300].replace("\n", " ")
        lines.append(f"  - {c}")
    return "\n".join(lines)


def persist(path: str, content: str, kind: str, project: str | None, agent_id: str) -> str:
    """Guarda el resultado del dispatch en Xavier.

    Normaliza el path para que SIEMPRE viva bajo dispatch/{project}/... (asi recall con
    path_prefix lo encuentra). Si el path ya empieza con 'dispatch/', se respeta.
    """
    proj = project or "global"
    if not path.startswith("dispatch/"):
        path = f"dispatch/{proj}/{path}"
    body = {
        "path": path,
        "content": content,
        "metadata": {
            "kind": kind,
            "namespace": {"project": proj, "agent_id": agent_id},
        },
    }
    r = _http("/memory/add", body)
    return r.get("id", r.get("_error", "?"))


# ---------------------------------------------------------------------------
# AGI CLI dispatcher (sincrono)
# ---------------------------------------------------------------------------
CLI_RUNNERS = {
    "opencode": {"config": "opencode.mcp.json", "cmd": "opencode"},
    "gemini": {"config": "gemini.settings.json", "cmd": "gemini"},
    "claude": {"config": "claude.mcp.json", "cmd": "claude"},
    "codex": {"config": "codex.system-prompt.md", "cmd": "codex"},
}


def _resolve_cli(name: str) -> str:
    resolved = shutil.which(name)
    if resolved:
        return resolved
    if os.name == "nt":
        npm_bin = os.path.expanduser("~/AppData/Roaming/npm")
        for ext in (".cmd", ".ps1", ""):
            cand = os.path.join(npm_bin, name + ext)
            if os.path.exists(cand):
                return cand
    return name


def dispatch_agi(cli: str, task: str, project: str | None, agent_id: str, timeout: int = 300) -> dict:
    if cli not in CLI_RUNNERS:
        return {"error": f"CLI desconocido: {cli}. Validos: {list(CLI_RUNNERS)}"}
    cfg_name = CLI_RUNNERS[cli]["config"]
    cfg_path = MCP_DIR / cfg_name
    if not cfg_path.exists():
        return {"error": f"Config MCP no existe: {cfg_path}. Ejecuta generate_mcp_configs.py primero."}

    # 1. RECALL
    print(f"[recall] buscando contexto previo en Xavier (project={project or 'global'})...")
    ctx = recall(task, project)
    print(f"[recall] {ctx.splitlines()[0]}")

    # 2. construir prompt con cerebro + contexto + tarea
    brain = (REPO_ROOT / "scripts" / "subagents" / "xavier_brain_prompt.md").read_text(encoding="utf-8")
    prompt = f"""{brain}

## Contexto recuperado de Xavier (USA esto)
{ctx}

## Tu identidad
- agent_id: {agent_id}
- project: {project or 'global'}

## Tarea
{task}

## Recordatorio
- Si necesitas MAS contexto, llama mem_search.
- Al terminar, llama create_memory con tu resultado (path="dispatch/{project or 'global'}/{agent_id}/resultado").
"""

    # 3. DISPATCH (subprocess)
    cmd = _resolve_cli(CLI_RUNNERS[cli]["cmd"])
    print(f"[dispatch] lanzando {cli} (cmd={cmd}, timeout={timeout}s)...")
    t0 = time.time()
    try:
        if cli == "opencode":
            import tempfile
            workdir = tempfile.mkdtemp(prefix="xavier-dispatch-")
            (Path(workdir) / "opencode.json").write_text(cfg_path.read_text(encoding="utf-8"), encoding="utf-8")
            r = subprocess.run([cmd, "run", "--auto", "--format", "json", "--dir", workdir, prompt],
                               capture_output=True, text=True, timeout=timeout)
        elif cli == "gemini":
            import tempfile
            workdir = tempfile.mkdtemp(prefix="xavier-dispatch-")
            gdir = Path(workdir) / ".gemini"; gdir.mkdir(exist_ok=True)
            (gdir / "settings.json").write_text(cfg_path.read_text(encoding="utf-8"), encoding="utf-8")
            r = subprocess.run([cmd, "-p", prompt, "-y"], capture_output=True, text=True, timeout=timeout, cwd=workdir)
        elif cli == "claude":
            r = subprocess.run([cmd, "-p", "--permission-mode", "bypassPermissions", "--mcp-config", str(cfg_path), prompt],
                               capture_output=True, text=True, timeout=timeout)
        else:  # codex (no MCP; el config es el system-prompt fallback)
            fallback = cfg_path.read_text(encoding="utf-8")
            r = subprocess.run([cmd, "exec", fallback + "\n\n" + prompt, "--dangerously-bypass-approvals-and-sandbox"],
                               capture_output=True, text=True, timeout=timeout)
        out = (r.stdout or "") + (r.stderr or "")
        rc = r.returncode
    except subprocess.TimeoutExpired:
        out, rc = "TIMEOUT", -1
    except FileNotFoundError:
        out, rc = f"CLI_NOT_FOUND: {cmd}", -2
    duration = round(time.time() - t0, 1)

    print(f"[dispatch] {cli} termino rc={rc} en {duration}s, output={len(out)} chars")

    # 4. PERSIST (guardar resultado del dispatch)
    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    persist_path = f"dispatch/{project or 'global'}/{agent_id}/{ts}"
    summary = f"Dispatch {cli} para tarea: {task[:200]}. Salida (ultimos 600 chars): ...{out[-600:]}"
    mem_id = persist(persist_path, summary, "session", project, agent_id)
    print(f"[persist] guardado en Xavier: {persist_path} (id={mem_id})")

    return {
        "type": "agi", "cli": cli, "agent_id": agent_id, "project": project,
        "task": task, "rc": rc, "duration_s": duration,
        "output_tail": out[-1000:], "persisted_path": persist_path, "persisted_id": mem_id,
        "ts": ts,
    }


# ---------------------------------------------------------------------------
# Jules dispatcher (DEPRECATED - Decommissioned)
# ---------------------------------------------------------------------------
def dispatch_jules(task: str = "", project: str | None = None, title: str | None = None, extra_labels: list[str] | None = None, repo: str = DEFAULT_REPO) -> dict:
    """[DEPRECATED] Jules service is decommissioned. Google Antigravity & agy CLI is the active canonical runner."""
    msg = (
        "Jules service has been decommissioned and removed from Xavier. "
        "Google Antigravity and the 'agy' CLI are the active canonical runners for all implementation waves and tasks."
    )
    print(f"\n[DEPRECATED] {msg}\n")
    return {
        "status": "deprecated",
        "error": msg,
        "active_runner": "Google Antigravity & agy CLI",
    }


def jules_status(repo: str = DEFAULT_REPO) -> dict:
    """[DEPRECATED] Jules service is decommissioned. Google Antigravity & agy CLI is the active canonical runner."""
    msg = (
        "Jules service has been decommissioned and removed from Xavier. "
        "Google Antigravity and the 'agy' CLI are the active canonical runners for all implementation waves and tasks."
    )
    print(f"\n[DEPRECATED] {msg}\n")
    return {
        "status": "deprecated",
        "error": msg,
        "active_runner": "Google Antigravity & agy CLI",
    }


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------
def main():
    ap = argparse.ArgumentParser(description="Xavier brain — subagent dispatcher (AGI CLIs + Antigravity)")
    sub = ap.add_subparsers(dest="cmd", required=True)

    p_agi = sub.add_parser("agi", help="Dispatch a un AGI CLI sincrono (codex/opencode/gemini/claude/qwen)")
    p_agi.add_argument("--cli", required=True, choices=list(CLI_RUNNERS))
    p_agi.add_argument("--task", required=True, help="Descripcion de la tarea")
    p_agi.add_argument("--project", default=None, help="Namespace project (aisla memoria)")
    p_agi.add_argument("--agent-id", default=None, help="Identidad del subagent (default: <cli>-dispatch)")
    p_agi.add_argument("--timeout", type=int, default=300, help="Timeout en segundos (AGI CLIs son sincronos)")

    p_jules = sub.add_parser("jules", help="[DEPRECATED] Jules is decommissioned; use Google Antigravity / agy")
    p_jules.add_argument("--task", default="", nargs="?", help="Descripcion de la tarea (deprecated)")
    p_jules.add_argument("--title", default=None, help="Titulo (deprecated)")
    p_jules.add_argument("--project", default=None, help="Namespace project (deprecated)")
    p_jules.add_argument("--label", action="append", default=[], help="Labels extra (deprecated)")
    p_jules.add_argument("--repo", default=DEFAULT_REPO, help="Repo GitHub (deprecated)")

    p_status = sub.add_parser("jules-status", help="[DEPRECATED] Jules is decommissioned")
    p_status.add_argument("--repo", default=DEFAULT_REPO, help="Repo GitHub (deprecated)")

    args = ap.parse_args()

    if args.cmd == "agi":
        agent_id = args.agent_id or f"{args.cli}-dispatch"
        result = dispatch_agi(args.cli, args.task, args.project, agent_id, args.timeout)
    elif args.cmd == "jules":
        result = dispatch_jules(args.task, args.project, args.title, args.label, args.repo)
    elif args.cmd == "jules-status":
        result = jules_status(args.repo)

    # guardar reporte
    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    report = REPORTS_DIR / f"dispatch_{args.cmd}_{ts}.json"
    report.write_text(json.dumps(result, indent=2, ensure_ascii=False, default=str), encoding="utf-8")
    print(f"\n[report] {report}")
    print(json.dumps(result, indent=2, ensure_ascii=False, default=str))


if __name__ == "__main__":
    main()
