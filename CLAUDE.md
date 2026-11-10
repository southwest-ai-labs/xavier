# CLAUDE.md — Xavier

> Puente para Claude Code. La fuente de verdad de las reglas es
> [`AGENTS.md`](AGENTS.md); este fichero existe porque Claude Code carga
> `CLAUDE.md` y `.claude/rules/`, y **no** lee `AGENTS.md` ni `.cursorrules`.
> Si una regla aparece en los dos sitios, `AGENTS.md` manda.
> No dupliques reglas aquí: pon el enlace y lo específico de Claude Code.

## Qué es esto

Motor de memoria vectorial para agentes de IA, en Rust. SQLite + `sqlite-vec`,
BM25, búsqueda híbrida. Tres superficies: HTTP, CLI y MCP. Nodo local y tres
endpoints SaaS. `apps/xavier` es **público**: los mensajes de commit y PR son
genéricos; no se describe un vector de ataque antes de cerrar la exposición real.

## Comandos de verificación

Estos y **solo** estos. Cada criterio de "hecho" del proyecto es la salida de uno.

```bash
cargo fmt -p xavier
cargo clippy -p xavier --all-targets --features ci-safe -- -D warnings
cargo test  -p xavier --features ci-safe --lib -- --test-threads=1
```

- `pnpm run lint` / `pnpm run build` son de `panel-ui`, **no** del crate Rust.
- **Nunca `--release`** en el ciclo de tests: 10-15 min por ciclo. Para desplegar,
  `cargo build -p xavier --release --bin xavier` **sin** `--features ci-safe`:
  un binario `ci-safe` pierde TUI, notificaciones y panel, y no es intercambiable
  con el desplegado.
- Baseline del ciclo actual (main 082b4072, 2026-10-09): **3469 pasan, 0 fallan, 1 ignorado**.

## Reglas del repo

- 1 PR = 1 feature. El ledger (`.gitcore/features.json`) lo promueve un run
  verde, **nunca a mano**.
- `thiserror` en libs, `anyhow` en binarios. Comentarios en inglés, densidad mínima.
- **Nunca** Rayon `.par_iter()` dentro de un worker Tokio → `tokio::task::spawn_blocking`.
- No commitear: `.env`, SQLite, WAL, SHM, logs, estado de sesión.
- `cargo fmt` obligatorio; clippy sin warnings (`-D warnings`).
- No hacer drive-by refactors, renames ni reformateos al tocar un fichero.

## Tres trampas de este repo

Estas no son opiniones; cada una costó una sesión.

1. **NUNCA `git add -A`.** `config/xavier.config.json` (hoy en `.gitignore`) y el estado
   de ejecución que el daemon reescribe (`~/.local/state/xavier/xavier.runtime.json`, que
   **manda** sobre `config/` desde #2801). Un `add -A` mete el estado de ejecución del
   daemon en un commit de un repo público. Ver issues #2801 y #2805.
2. **Un test que lea un fichero mutable no es un test.** `settings::tests::test_load_config_json`
   (settings/mod.rs:249) afirma sobre `config/xavier.config.json`, que el daemon
   cambia. Por eso «3469 pasan» significa cosas distintas según el estado del
   daemon. Usa `tempdir` y raíz inyectada siempre.
3. **Esta máquina ES el nodo de producción.** La BD de producción es
   `data/vec-store.sqlite3` (1,45 GB) y las fuentes del arnés son de GB.
   Los tests jamás las abren: solo lectura, y solo con tempdir.

## Cómo se demuestra que un fix funciona

Un test verde **no** cuenta. Hace falta:

1. **Mutation test.** Retira la mitigación y demuestra que el test se pone rojo.
   Reporta el número exacto de tests rojos. Un test que no falla al retirar lo que
   protege no está probando nada.
2. **Proceso real.** Nada se cierra hasta que el binario desplegado lo ejecuta y
   se mide el efecto. Si producción contradice al test, gana la medición.
3. **Wiring real.** Los tests de autorización se exercise sobre el router
   productivo, nunca sobre una copia local.

## Specs y ADRs

`docs/adr/` (context → decision → consequences), `docs/SRS/REQUIREMENTS.md`
(IEEE 830). Una decisión no obvia es un ADR; una que contradice un ADR lo
actualiza o abre uno nuevo. `.gitcore/sdd/` es spec efímera y se borra sin
tocar `features.json`.

## Antes de tocar código de memoria o consolidación

- `src/consolidation/` (MinHash, decay, importance, TGD) **sí se ejecutaba cada noche
  (21:00 COT) desde 2026-09-27 y BORRÓ ~277 registros** (fase 1, sin LLM). Apagado el
  2026-10-12 en el runtime overlay; PR #2994 lo hace opt-in (`XAVIER_TGD_SCHEDULER`). No escribas un sistema
  nuevo: léelo y conéctalo.
- `src/memory/tgd.rs` → `TgdUtilityPruner` pregunta «¿esto alguna vez hizo
  falta?». Depende de `last_accessed_at` / `access_count`, que hasta ahora no
  estaban poblados: sin esa señal, podar es adivinar.
- La ingesta de sesiones es un **canal de inyección de prompts**: lo ingerido
  vuelve al contexto de agentes con herramientas. Cualquier endpoint de
  mantenimiento es **solo HTTP-admin**, nunca MCP.

## Entorno de producción (no tocar sin pedir)

- Daemon: `systemctl --user xavier.service`, HTTP `127.0.0.1:8006`, MCP `127.0.0.1:8100`.
- Ingesta periódica **reactivada** el 2026-10-12 (`XAVIER_INGESTION_INTERVAL_SECS=600`,
  drop-in `diag-ingestion.conf`). Medición: con cursores persistidos (`data/ingest-cursors/`)
  un ciclo tarda 0,14 s con `read=0` en las 4 fuentes y 3 % de CPU (antes, busy-loop
  de 78-94 %). Si se borran los cursores, la primera pasada completa tarda ~67 min a
  1 núcleo. `/proc/<pid>/io` no es legible (proceso no volcable): se mide la CPU por
  `cpu.stat` del cgroup.
- La compactación semántica programada es **opt-in** (`XAVIER_SEMANTIC_COMPACTION=1`)
  desde #2978: el 2026-10-12 truncó 1105 memorias en producción sin modelo de
  compactación disponible. No activarla sin un modelo verificado.
- Backups: `~/.hermes/scripts/xavier-backup-*.sh` + timer `xavier-backup.timer`
  (03:17). Usan `VACUUM INTO` porque `iterdump()` destruye el índice FTS5.
  **No** cambiar a `.dump`.
- `master.key` es material criptográfico: backup verificado y migración
  reversible antes de tocarlo.
