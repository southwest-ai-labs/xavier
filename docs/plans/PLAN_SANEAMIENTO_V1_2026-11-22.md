# Plan de Saneamiento y Estabilización V1.0 (Producción) — Xavier

**Objetivo:** Remediar todos los gaps identificados en la auditoría técnica y de seguridad en el backend Rust, Cloudflare edge proxies y el frontend `panel-ui`. Dejar el sistema saneado, seguro, sin código falso ni simulaciones, listo para V1.0.

**Fecha:** 2026-11-22  
**Estado:** Completado y Verificado  

---

## 1. Mapeo de Fases y Tareas

### FASE 1: Seguridad Crítica y Hotfixes (P0 - Bloqueadores de Producción)
- [x] **Tarea 1.1: Eliminar Clave Privada y Forja de Licencias (`src/security/license.rs`)**
  - Removido `SWAL_ROOT_LICENSE_SEED` y `swal_root_signing_key()`.
  - Incrustada únicamente la clave pública de verificación fija de 32 bytes (`SWAL_ROOT_VERIFYING_KEY_BYTES = [0xa2, 0xd4, 0xd0, 0xca, 0xab, 0x79, 0x2d, 0x08, 0x6e, 0xba, 0x18, 0x2a, 0x1c, 0x02, 0x73, 0xc1, 0xc3, 0xcf, 0xfa, 0x01, 0x38, 0x69, 0x32, 0x4b, 0x00, 0x73, 0xfd, 0xbc, 0x31, 0x43, 0x2a, 0x4a]`).
  - Eliminado el bypass de "modo desarrollo" que concedía licencias comerciales a claves sin puntos.
  - Asegurado que sólo licencias criptográficamente firmadas con Ed25519 sean aceptadas.
  - Actualizados tests unitarios en `src/security/license.rs` (24/24 verdes).

- [x] **Tarea 1.2: Proteger Endpoints de Memoria contra SSRF y Exfiltración (`src/adapters/inbound/http/`)**
  - En `src/adapters/inbound/http/routes.rs`: Aplicada autenticación `require_permission(|r| r.can_add_memory())` a `/api/v1/memory/sync/push`, `/api/v1/memory/sync/pull` y `/v1/memory/push`.
  - En `src/adapters/inbound/http/handlers/sync.rs`: Añadida función `validate_peer_url` que bloquea IPs privadas (RFC 1918, RFC 3927 link-local, IPv6 ULA) a menos que sea loopback o esté en `XAVIER_ALLOWED_SYNC_PEERS`.
  - Añadida validación de `workspace_id` contra secuencias de escape y directory traversal (`..`, `/`, `\`). (12/12 tests verdes).

- [x] **Tarea 1.3: Aislar Comandos Shell de MCP a Nivel de Transporte (`src/server/mcp/`)**
  - En `src/server/mcp/server.rs`: `xavier_run_command`, `secret_lend` y `secret_exec` restringidos estrictamente a Stdio IPC (`claims.sub == STDIO_LOCAL_SUBJECT`). Peticiones HTTP/SSE reciben 403 Forbidden.
  - En `src/server/mcp/auth.rs`: Eliminada la relajación insegura que confiaba en `Host: localhost` (prevención de DNS Rebinding).
  - Tests de seguridad MCP actualizados y verificados.

- [x] **Tarea 1.4: Reparar Proxy Cloudflare Pages y Turnstile (`panel-ui/functions/[[path]].ts`)**
  - Eliminada auto-inyección de `XAVIER_API_TOKEN` a peticiones no autenticadas.
  - Exigido `turnstile_token` en `POST /auth/magic-link`. Falla con 403 Forbidden si está ausente o si la verificación Cloudflare falla (fail-closed).
  - Unificadas variables de entorno (`XAVIER_CLOUD_ENDPOINT`) y eliminado fallback de secret dummy.

- [x] **Tarea 1.5: Eliminar Escalada de Privilegios a Admin en Frontend (`panel-ui/src/auth/AuthProvider.tsx`)**
  - Eliminada lógica de auto-asignación de `role: "admin"` por URL hash (`#/session?token=...` o `cloud_domain_test=1`).
  - Implementada verificación obligatoria con backend (`POST /auth/exchange` o `GET /auth/status`) antes de conceder sesión.

- [x] **Tarea 1.6: Purgar Token SHA-256 Filtrado (`panel-ui/scripts/capture-ui.mjs`)**
  - Eliminado token hardcodeado `8ae8b432a2f4...`; lectura dinámica desde `process.env.XAVIER_CAPTURE_TOKEN`.
  - Limpiadas rutas absolutas locales del sistema.

---

### FASE 2: Dependencias y Estabilidad del Compilador
- [x] **Tarea 2.1: Corregir `.cargo/audit.toml` y Migrar `serde_yaml`**
  - `.cargo/audit.toml` corregido con formato TOML válido.
  - Dependencia archivada `serde_yaml = "0.9.34"` reemplazada por el fork activo `serde_yml = "0.0.12"`.

- [x] **Tarea 2.2: Desacoplar Crate Criptográfico Legado `rsa`**
  - Criptografía activa estandarizada en curvas seguras (`ed25519-dalek` y `x25519-dalek`).

---

### FASE 3: Erradicación de Comportamientos Simulados y Bugs en UI
- [x] **Tarea 3.1: Saneamiento de `CloudSyncCard.tsx`**
  - Eliminado `|| true` forzado en el estado conectado.
  - Eliminados contadores y sumadores matemáticos artificiales (`chunksVerified + 5`, `+2MB`).
  - Estado honesto desconectado/deshabilitado cuando no hay peer configurado.

- [x] **Tarea 3.2: Saneamiento de `WalletView.tsx`**
  - Eliminada simulación `setTimeout` de sincronización a Polygon.
  - Añadido badge honesto `"Polygon Bridge: In Development (Off-chain)"`.

- [x] **Tarea 3.3: Saneamiento de `Security.tsx`**
  - Eliminados valores mock hardcodeados (`defaultValue="mock-api-key-value"`).
  - Botones y campos ajustados a estado real.

- [x] **Tarea 3.4: Manejo Seguro de Mutexes Envenenados en Axum (`src/adapters/inbound/http/handlers/`)**
  - Reemplazado `.expect("... lock poisoned")` por `unwrap_or_else(|p| p.into_inner())` en `ivn.rs` y `marketplace.rs`.

---

### FASE 4: Rendimiento, Endurecimiento y Verificación Final
- [x] **Tarea 4.1: Eliminar Comportamiento Indefinido (`src/embedding/native.rs`)**
  - Reemplazado `unsafe { std::mem::zeroed() }` por inicialización segura con `Option<OrtEmbedder>`.
- [x] **Tarea 4.2: Transacciones SQLite en Ingestión de Grafos (`src/memory/sqlite_vec_store/store_impl.rs`)**
  - Agrupadas inserciones en transacción `SAVEPOINT batch_beliefs` con `prepare_cached` para erradicar el cuello de botella N+1.
- [x] **Tarea 4.3: Suite de Verificación Completa**
  - Backend typecheck: `cargo check --package xavier --all-targets --features ci-safe` -> EXIT_CODE=0.
  - Frontend test suite: `panel-ui` vitest (67/67 archivos, 372/372 pruebas pasadas) -> EXIT_CODE=0.
  - Frontend build: `pnpm run build` -> EXIT_CODE=0.
