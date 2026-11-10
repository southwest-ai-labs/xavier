#!/usr/bin/env python3
"""
Generate embeddings for all records in memory_records missing from memory_embeddings_768.
Uses local Ollama nomic-embed-text (768 dims) in batches.
"""

import sys
import json
import struct
import urllib.request
import sqlite3
from pathlib import Path

DB_PATH = Path("/home/belal/proyectosSWAL/apps/xavier/data/vec-store.sqlite3")
OLLAMA_URL = "http://localhost:11434/v1/embeddings"
BATCH_SIZE = 25

def get_embeddings_batch(texts):
    payload = {
        "model": "nomic-embed-text",
        "input": texts
    }
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        OLLAMA_URL,
        data=data,
        headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        res = json.loads(resp.read().decode("utf-8"))
        return [item["embedding"] for item in res["data"]]

def main():
    if not DB_PATH.exists():
        print(f"Database not found at {DB_PATH}")
        sys.exit(1)

    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("""
        SELECT id, workspace_id, path, content, metadata
        FROM memory_records
        WHERE id NOT IN (SELECT id FROM memory_embeddings_768)
    """)
    records = cursor.fetchall()
    total = len(records)
    print(f"Found {total} records missing embeddings.")

    if total == 0:
        print("All records already have embeddings.")
        conn.close()
        return

    processed = 0
    for i in range(0, total, BATCH_SIZE):
        batch = records[i:i + BATCH_SIZE]
        texts_to_embed = []
        for r in batch:
            doc_id, ws, path, content, meta_str = r
            title = ""
            if meta_str:
                try:
                    meta = json.loads(meta_str)
                    title = meta.get("title", "")
                except Exception:
                    pass
            header = f"{title}\n{path}\n" if title else f"{path}\n"
            text = header + (content[:1200] if content else "")
            texts_to_embed.append(text[:2000])

        try:
            vectors = get_embeddings_batch(texts_to_embed)
            insert_rows = []
            for (doc_id, ws, _, _, _), vec in zip(batch, vectors):
                blob = struct.pack(f"{len(vec)}f", *vec)
                insert_rows.append((doc_id, ws or "default", blob))

            cursor.executemany("""
                INSERT OR REPLACE INTO memory_embeddings_768 (id, workspace_id, embedding)
                VALUES (?, ?, ?)
            """, insert_rows)
            conn.commit()

            processed += len(batch)
            print(f"Embedded {processed}/{total} ({processed*100//total}%)")
        except Exception as e:
            print(f"Error in batch {i}: {e}")

    conn.close()
    print("Embedding generation completed successfully.")

if __name__ == "__main__":
    main()
