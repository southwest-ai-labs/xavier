#!/usr/bin/env python3
"""
Xavier Codebase & Documentation Indexer
Indexes documentation and key codebase files from SWAL projects and apps/xavier
into Xavier's vector memory store with authentic file timestamps and metadata.
"""

import os
import sys
import json
import time
import sqlite3
import datetime
from pathlib import Path

SWAL_ROOT = Path("/home/belal/proyectosSWAL")
XAVIER_ROOT = SWAL_ROOT / "apps" / "xavier"
DB_PATH = XAVIER_ROOT / "data" / "vec-store.sqlite3"

def generate_ulid():
    """Generates a sortable ULID string compatible with Xavier."""
    t = int(time.time() * 1000)
    # 10 chars time, 16 chars random
    time_chars = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    res = []
    for _ in range(10):
        res.append(time_chars[t % 32])
        t //= 32
    time_part = "".join(reversed(res))
    import random
    rand_part = "".join(random.choice(time_chars) for _ in range(16))
    return f"01{time_part[2:]}{rand_part}"

def extract_title(content: str, default_name: str) -> str:
    """Extracts first markdown heading # Title or returns default."""
    for line in content.splitlines():
        line = line.strip()
        if line.startswith("# "):
            return line[2:].strip()
    return default_name

def split_sections(content: str, max_chunk_words: int = 500):
    """Splits markdown by ## headings into logical chunks."""
    sections = []
    current_title = "Overview"
    current_lines = []
    
    for line in content.splitlines():
        if line.startswith("# ") or line.startswith("## "):
            if current_lines:
                text = "\n".join(current_lines).strip()
                if len(text.split()) > 20:
                    sections.append((current_title, text))
            current_title = line.lstrip("#").strip()
            current_lines = [line]
        else:
            current_lines.append(line)
            
    if current_lines:
        text = "\n".join(current_lines).strip()
        if len(text.split()) > 20:
            sections.append((current_title, text))
            
    return sections

def get_target_files():
    """Finds all high-value markdown and config files to index."""
    files = []
    
    # 1. Xavier Root Docs
    xavier_targets = [
        "README.md",
        "AGENTS.md",
        "CHANGELOG.md",
        "THIRD_PARTY.md",
        "Cargo.toml",
    ]
    for rel in xavier_targets:
        p = XAVIER_ROOT / rel
        if p.exists():
            files.append((p, f"apps/xavier/{rel}", "xavier"))
            
    # Xavier docs and gitcore
    for d in ["docs", ".gitcore", "crates"]:
        dir_path = XAVIER_ROOT / d
        if dir_path.exists():
            for root, _, f_names in os.walk(dir_path):
                for f in f_names:
                    if f.endswith(".md") or f in ["features.json", "MANIFEST.json"]:
                        full = Path(root) / f
                        rel = full.relative_to(XAVIER_ROOT)
                        files.append((full, f"apps/xavier/{rel}", "xavier"))

    # 2. SWAL Ecosystem Root Docs
    swal_targets = [
        "README.md",
        "AGENTS.md",
        "AGENTS_HERMES.md",
        "modelo-negocio-xavier.md",
        "agent-registry.toml",
    ]
    for rel in swal_targets:
        p = SWAL_ROOT / rel
        if p.exists():
            files.append((p, f"proyectosSWAL/{rel}", "swal-ecosystem"))
            
    # SWAL docs
    swal_docs = SWAL_ROOT / "docs"
    if swal_docs.exists():
        for root, _, f_names in os.walk(swal_docs):
            if "node_modules" in root or ".git" in root or "private" in root:
                continue
            for f in f_names:
                if f.endswith(".md"):
                    full = Path(root) / f
                    rel = full.relative_to(SWAL_ROOT)
                    files.append((full, f"proyectosSWAL/{rel}", "swal-ecosystem"))

    # 3. Sibling SWAL Apps READMEs
    for app_dir in (SWAL_ROOT / "apps").iterdir():
        if app_dir.is_dir() and app_dir.name != "xavier":
            for rel in ["README.md", "AGENTS.md"]:
                p = app_dir / rel
                if p.exists():
                    files.append((p, f"apps/{app_dir.name}/{rel}", app_dir.name))
                    
    # 4. Core Libraries
    for core_dir in (SWAL_ROOT / "cores").iterdir():
        if core_dir.is_dir():
            for rel in ["README.md", "AGENTS.md"]:
                p = core_dir / rel
                if p.exists():
                    files.append((p, f"cores/{core_dir.name}/{rel}", core_dir.name))

    return files

def index_codebase():
    if not DB_PATH.exists():
        print(f"Error: Database not found at {DB_PATH}")
        sys.exit(1)
        
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    
    files = get_target_files()
    print(f"Found {len(files)} target files to index from codebase.")
    
    indexed_count = 0
    now_iso = datetime.datetime.now(datetime.timezone.utc).isoformat()
    
    for full_path, rel_path, project in files:
        try:
            stat = full_path.stat()
            mtime_iso = datetime.datetime.fromtimestamp(stat.st_mtime, tz=datetime.timezone.utc).isoformat()
            ctime_iso = datetime.datetime.fromtimestamp(stat.st_ctime, tz=datetime.timezone.utc).isoformat()
            
            with open(full_path, "r", encoding="utf-8", errors="replace") as f:
                content = f.read()
                
            if not content.strip():
                continue
                
            doc_title = extract_title(content, full_path.name)
            
            # Check if this exact file path already indexed
            cursor.execute("SELECT id FROM memory_records WHERE path = ?", (rel_path,))
            existing = cursor.fetchone()
            
            metadata = {
                "source": "codebase",
                "kind": "document",
                "type": "documentation" if rel_path.endswith(".md") else "config",
                "file": full_path.name,
                "file_path": str(full_path),
                "title": doc_title,
                "last_modified": mtime_iso,
                "created_at": ctime_iso,
                "project": project,
                "namespace": {
                    "workspace_id": "default",
                    "project": project
                },
                "provenance": {
                    "file_path": str(full_path),
                    "source_type": "codebase"
                }
            }
            
            meta_json = json.dumps(metadata)
            
            if existing:
                doc_id = existing[0]
                cursor.execute("""
                    UPDATE memory_records
                    SET content = ?, metadata = ?, updated_at = ?, primary_flag = 1
                    WHERE id = ?
                """, (content, meta_json, mtime_iso, doc_id))
            else:
                doc_id = generate_ulid()
                cursor.execute("""
                    INSERT INTO memory_records (
                        id, workspace_id, path, content, metadata,
                        created_at, updated_at, revision, primary_flag,
                        level, embedding_status
                    ) VALUES (?, 'default', ?, ?, ?, ?, ?, 1, 1, 'atom', 'pending')
                """, (doc_id, rel_path, content, meta_json, ctime_iso, mtime_iso))
                
            # Update FTS
            cursor.execute("DELETE FROM memory_fts WHERE id = ?", (doc_id,))
            cursor.execute("""
                INSERT INTO memory_fts (id, path, content, code_tokens)
                VALUES (?, ?, ?, ?)
            """, (doc_id, rel_path, content, f"{rel_path} {doc_title}"))
            
            indexed_count += 1
            
        except Exception as e:
            print(f"Error indexing {full_path}: {e}")
            
    conn.commit()
    conn.close()
    print(f"Successfully indexed/updated {indexed_count} codebase documents in {DB_PATH}")

if __name__ == "__main__":
    index_codebase()
