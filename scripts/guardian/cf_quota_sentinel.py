#!/usr/bin/env python3
"""cf_quota_sentinel.py — Cloudflare Quota & Sentinel Guard for SWAL / Xavier.

Can be run:
  1. Locally on systemd user timer (cf-quota-sentinel.timer)
  2. In GitHub Actions (.github/workflows/cf-quota-sentinel.yml)
Zero Cloudflare cost ($0).
Alerts via Xavier Telegram Bot when thresholds (>= 700 writes/day) are reached.
"""
from __future__ import annotations

import argparse
import datetime
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ACCOUNT_ID = "963f01052b7f84cb785e72ba2b4d6e12"
GRAPHQL_URL = "https://api.cloudflare.com/client/v4/graphql"
XAVIER_URL = os.environ.get("XAVIER_URL", "http://localhost:8006")

LIMITS = {
    "kv_writes": 1000,
    "kv_deletes": 1000,
    "kv_reads": 100000,
    "workers_requests": 100000,
}

WARN_THRESHOLDS = {
    "kv_writes": 700,
    "kv_deletes": 700,
    "kv_reads": 80000,
    "workers_requests": 80000,
}


def load_env_var(name: str) -> str | None:
    val = os.environ.get(name)
    if val:
        return val.strip("\"' ")
    for env_path in [
        Path.home() / ".hermes" / ".env",
        Path.home() / "proyectosSWAL" / "apps" / "xavier" / ".env",
    ]:
        if env_path.exists():
            for line in env_path.read_text().splitlines():
                line = line.strip()
                if line.startswith(f"{name}="):
                    return line.split("=", 1)[1].strip("\"' ")
    return None


def load_cf_token() -> str:
    token = load_env_var("CLOUDFLARE_API_TOKEN") or load_env_var("CF_API_TOKEN")
    if token:
        return token
    wrangler_file = Path.home() / ".wrangler" / "config" / "default.toml"
    if wrangler_file.exists():
        import re
        m = re.search(r'oauth_token\s*=\s*"([^"]+)"', wrangler_file.read_text())
        if m:
            return m.group(1)
    raise RuntimeError("No Cloudflare API token found in ENV, ~/.hermes/.env or ~/.wrangler")


def query_graphql(token: str, query: str, variables: dict) -> dict:
    req = urllib.request.Request(
        GRAPHQL_URL,
        data=json.dumps({"query": query, "variables": variables}).encode("utf-8"),
        headers={
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
            "User-Agent": "SWAL-Xavier-Sentinel/1.0",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def audit_cloudflare(account_id: str = ACCOUNT_ID) -> dict:
    token = load_cf_token()
    now = datetime.datetime.now(datetime.timezone.utc)
    one_day_ago = now - datetime.timedelta(hours=24)
    start_str = one_day_ago.strftime("%Y-%m-%dT%H:%M:%SZ")
    end_str = now.strftime("%Y-%m-%dT%H:%M:%SZ")

    kv_query = """
    query GetKVStats($accountTag: String!, $start: String!, $end: String!) {
      viewer {
        accounts(filter: {accountTag: $accountTag}) {
          kvOperationsAdaptiveGroups(
            filter: {datetime_geq: $start, datetime_leq: $end}
            limit: 100
          ) {
            dimensions {
              namespaceId
              actionType
            }
            sum {
              requests
            }
          }
        }
      }
    }
    """
    kv_res = query_graphql(token, kv_query, {
        "accountTag": account_id,
        "start": start_str,
        "end": end_str,
    })

    total_writes = 0
    total_reads = 0
    total_deletes = 0
    by_namespace = {}

    accounts = kv_res.get("data", {}).get("viewer", {}).get("accounts", [])
    if accounts:
        groups = accounts[0].get("kvOperationsAdaptiveGroups", [])
        for g in groups:
            dim = g.get("dimensions", {})
            ns = dim.get("namespaceId", "unknown")
            action = dim.get("actionType", "unknown")
            reqs = g.get("sum", {}).get("requests", 0)

            if ns not in by_namespace:
                by_namespace[ns] = {"writes": 0, "reads": 0, "deletes": 0}

            if action == "write":
                total_writes += reqs
                by_namespace[ns]["writes"] += reqs
            elif action == "read":
                total_reads += reqs
                by_namespace[ns]["reads"] += reqs
            elif action == "delete":
                total_deletes += reqs
                by_namespace[ns]["deletes"] += reqs

    workers_query = """
    query GetWorkerStats($accountTag: String!, $start: String!, $end: String!) {
      viewer {
        accounts(filter: {accountTag: $accountTag}) {
          workersInvocationsAdaptive(
            filter: {datetime_geq: $start, datetime_leq: $end}
            limit: 100
          ) {
            dimensions {
              scriptName
            }
            sum {
              requests
            }
          }
        }
      }
    }
    """
    workers_res = query_graphql(token, workers_query, {
        "accountTag": account_id,
        "start": start_str,
        "end": end_str,
    })

    total_worker_requests = 0
    by_worker = {}
    if workers_res.get("data", {}).get("viewer", {}).get("accounts", []):
        w_groups = workers_res["data"]["viewer"]["accounts"][0].get("workersInvocationsAdaptive", [])
        for wg in w_groups:
            script = wg.get("dimensions", {}).get("scriptName", "unknown")
            reqs = wg.get("sum", {}).get("requests", 0)
            total_worker_requests += reqs
            by_worker[script] = reqs

    status = "OK"
    warnings = []
    if total_writes >= WARN_THRESHOLDS["kv_writes"]:
        status = "CRITICAL" if total_writes >= LIMITS["kv_writes"] else "WARN"
        warnings.append(f"KV writes ({total_writes}) near/exceeds daily budget ({LIMITS['kv_writes']})")

    if total_deletes >= WARN_THRESHOLDS["kv_deletes"]:
        status = "CRITICAL" if total_deletes >= LIMITS["kv_deletes"] else "WARN"
        warnings.append(f"KV deletes ({total_deletes}) near/exceeds daily budget ({LIMITS['kv_deletes']})")

    if total_worker_requests >= WARN_THRESHOLDS["workers_requests"]:
        status = "CRITICAL" if total_worker_requests >= LIMITS["workers_requests"] else "WARN"
        warnings.append(f"Worker requests ({total_worker_requests}) near limit")

    report = {
        "timestamp": now.isoformat(),
        "status": status,
        "account_id": account_id,
        "metrics_24h": {
            "kv_writes": total_writes,
            "kv_writes_limit": LIMITS["kv_writes"],
            "kv_writes_pct": round(total_writes / LIMITS["kv_writes"] * 100, 1),
            "kv_deletes": total_deletes,
            "kv_deletes_limit": LIMITS["kv_deletes"],
            "kv_reads": total_reads,
            "kv_reads_limit": LIMITS["kv_reads"],
            "workers_requests": total_worker_requests,
            "workers_requests_limit": LIMITS["workers_requests"],
            "workers_requests_pct": round(total_worker_requests / LIMITS["workers_requests"] * 100, 1),
        },
        "by_namespace": by_namespace,
        "by_worker": by_worker,
        "warnings": warnings,
    }
    return report


def send_telegram_alert(report: dict) -> bool:
    bot_token = load_env_var("TELEGRAM_BOT_TOKEN")
    chat_id = load_env_var("TELEGRAM_HOME_CHANNEL") or load_env_var("TELEGRAM_CHAT_ID") or load_env_var("TELEGRAM_ALLOWED_USERS")
    if not bot_token or not chat_id:
        return False

    status_emoji = "🚨" if report["status"] == "CRITICAL" else ("⚠️" if report["status"] == "WARN" else "✅")
    m = report["metrics_24h"]
    msg_lines = [
        f"{status_emoji} *SWAL Cloudflare Sentinel Alert*",
        f"*Status:* `{report['status']}`",
        f"*KV Writes (24h):* `{m['kv_writes']}` / `{m['kv_writes_limit']}` ({m['kv_writes_pct']}%)",
        f"*KV Deletes (24h):* `{m['kv_deletes']}` / `{m['kv_deletes_limit']}`",
        f"*KV Reads (24h):* `{m['kv_reads']}`",
        f"*Workers Invocations:* `{m['workers_requests']}` / `{m['workers_requests_limit']}`",
    ]
    if report["warnings"]:
        msg_lines.append("\n*Warnings:*")
        for w in report["warnings"]:
            msg_lines.append(f"• {w}")

    # Top consumers
    top_ns = sorted(report["by_namespace"].items(), key=lambda x: x[1]["writes"], reverse=True)[:3]
    if top_ns and top_ns[0][1]["writes"] > 0:
        msg_lines.append("\n*Top KV Namespaces by Writes:*")
        for ns_id, stats in top_ns:
            if stats["writes"] > 0:
                msg_lines.append(f"• `{ns_id[:16]}...`: {stats['writes']} writes, {stats['reads']} reads")

    text = "\n".join(msg_lines)
    url = f"https://api.telegram.org/bot{bot_token}/sendMessage"
    payload = json.dumps({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "Markdown",
        "disable_web_page_preview": True,
    }).encode("utf-8")

    try:
        req = urllib.request.Request(
            url,
            data=payload,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status in (200, 201)
    except Exception as e:
        print(f"[WARN] Failed to send Telegram alert: {e}", file=sys.stderr)
        return False


def sync_xavier_memory(report: dict) -> bool:
    token = load_env_var("XAVIER_TOKEN")
    if not token:
        return False
    try:
        m = report["metrics_24h"]
        content = (
            f"Cloudflare Quota Sentinel Report [{report['status']}]: "
            f"KV writes 24h: {m['kv_writes']}/{m['kv_writes_limit']} ({m['kv_writes_pct']}%), "
            f"KV reads 24h: {m['kv_reads']}, "
            f"Worker requests 24h: {m['workers_requests']}/{m['workers_requests_limit']}. "
            f"Warnings: {len(report['warnings'])}"
        )
        ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%d-%H%M")
        payload = json.dumps({
            "path": f"swal/cloudflare/sentinel/status-{ts}.json",
            "content": content,
            "metadata": {
                "project": "swal",
                "type": "infrastructure_quota",
                "status": report["status"],
                "kv_writes": m["kv_writes"],
            },
        }).encode("utf-8")

        req = urllib.request.Request(
            f"{XAVIER_URL}/memory/add",
            data=payload,
            headers={
                "Content-Type": "application/json",
                "X-Xavier-Token": token,
                "Authorization": f"Bearer {token}",
            },
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=5) as r:
            return r.status in (200, 201)
    except Exception:
        return False


def main():
    parser = argparse.ArgumentParser(description="SWAL Cloudflare Quota Sentinel")
    parser.add_argument("--notify", action="store_true", help="Always send Telegram notification even if OK")
    parser.add_argument("--json", action="store_true", help="Output JSON format")
    parser.add_argument("--status-file", type=str, default=None, help="Path to write status JSON")
    args = parser.parse_args()

    try:
        report = audit_cloudflare()
    except Exception as e:
        print(f"[ERROR] Sentinel audit failed: {e}", file=sys.stderr)
        sys.exit(1)

    status_path = Path(args.status_file) if args.status_file else Path.home() / ".hermes" / "data" / "cf-quota-status.json"
    status_path.parent.mkdir(parents=True, exist_ok=True)
    status_path.write_text(json.dumps(report, indent=2))

    xavier_synced = sync_xavier_memory(report)

    # Notify Telegram if critical/warn or explicitly asked
    telegram_sent = False
    if report["status"] in ("WARN", "CRITICAL") or args.notify:
        telegram_sent = send_telegram_alert(report)

    if args.json:
        print(json.dumps(report, indent=2))
    else:
        m = report["metrics_24h"]
        print(
            f"[{report['status']}] CF Sentinel: KV writes={m['kv_writes']}/{m['kv_writes_limit']} ({m['kv_writes_pct']}%), "
            f"Worker reqs={m['workers_requests']} | Xavier sync={'OK' if xavier_synced else 'SKIPPED'} | Telegram={'SENT' if telegram_sent else 'NO'}"
        )
        if report["warnings"]:
            for w in report["warnings"]:
                print(f"  ⚠️  {w}")

    if report["status"] == "CRITICAL":
        sys.exit(2)
    sys.exit(0)


if __name__ == "__main__":
    main()
