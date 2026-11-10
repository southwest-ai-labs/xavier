# PLAN DE ACCIÓN — Navegabilidad para agentes

**Origen:** auditoría de 10 sesiones de agente (2026-10-02) + 125 repos.
**Fecha:** 2026-10-03 | **Estado:** propuesto, nada implementado aún.

Tres capas, de la más barata a la más cara, de la más barata a la más cara:

| Capa | Qué arregla | Coste |
|---|---|---|
| **A. Hermes / disciplina** | builds que no diagnostican, gasto de contexto | 0 código |
| **B. Docs de agente por repo** | symbols no encontrados, docs contradictorias | bajo |
| **C. Xavier** | el grafo que debe sostener todo lo anterior | ~1 sprint |

---

## Principio rector (esto decide el orden)

> **Un dato sin `project_root` no es un dato: es una hipótesis.**
> **Un build que no ha terminado no tiene diagnóstico: tiene ruido.**
> **Un doc que no nombra símbolos es prosa, no un mapa.**

Las tres frases son el criterio de aceptación de *todas* las tareas de abajo.
Si una tarea no puede verificarse contra ellas, no está bien hecha.

---

# CAPA A — Hermes (yo). Sin código.

## A1. Skill `agent-navigability-discipline` — YA ENCOLADA

Estado: pending `0585bb6e` (create) + `05ef2748` (patch). **Falta tu
`/skills approve`.** Hasta entonces yo la cargo leyendo el fichero, y los
subagentes no la ven.

Contenido: las 3 reglas, con evidencia medida.

**Acción tuya:** aprobar. Es el primer paso de todo lo demás — sin ella, las
capas B y C no tienen forma de FIXarse.

## A2. Cerrar la brecha `read_file` sin `limit`

42% de los `read_file` (26/62) volcaron el archivo entero para buscar un símbolo.
Eso son tokens quemados en texto que el modelo no puede citar.

**Acción:** al delegar, instruir explícitamente: *localiza la línea con grep,
luego `read_file` con `offset`/`limit`*. Entra en el prompt de delegación, no en
el skill.

## A3. `notify` en lugar de `wait`

34 `process_manage wait` bloqueantes. El runtime ya emite
`notify_on_complete`.

**Acción:** nunca `wait` en un oneshot; lanzar, seguir trabajando, leer el log.
Ya está en el skill (A1). **Acción tuya:** ninguna, es disciplina mía.

---

# CAPA B — Docs de agente por repo

## Estado medido

De 125 repos: 24 con 1 doc, 20 con 2, 4 con 3, **6 con 4**.
`.cursorrules` obsoleto en 9 repos (el peor: 166 días). `synapse-protocol`: 111
días de deriva, 180 LOC.
Prueba de fallos en xavier: **0 de 15** símbolos buscados aparecían en 720
líneas de docs.

## B1. Regla: un solo doc de entrada

`AGENTS.md` es la fuente. `CLAUDE.md` y `.cursorrules` se borran o bajan a un
puntero de 3 líneas. Cuatro ficheros que pueden contradecirse son peor que uno
que puede estar mal en un sitio.

**Alcance:** empezar por los 6 repos con 4 docs (`xavier`, `gara-g`, `GOS`,
`software-factory`, `synapse-protocol`, `CGP-Colegios`).

## B2. `AGENTS.md` debe llevar mapa de símbolos

Tabla `símbolo → fichero:línea` de los ~30 símbolos más buscados del repo.

**Ya aplicado en xavier** (15 símbolos, cada línea verificada releyendo el
fichero: 14/14 exactas). Plantilla en `B3`.

## B3. Test de regresión de docs

Falla si un símbolo del top-30 no aparece en `AGENTS.md`. Convierte la deriva
de docs en fallo de CI en vez de en turno perdido.

## B4. Generador del mapa (para no mantener 30 tablas a mano)

Script que extrae los símbolos más buscados de las sesiones reales
(`~/.hermes/state.db` → `messages.tool_calls` → patrones `search_files`), y
resuelve cada uno con `grep` al fichero y línea. Genera la tabla; el humano
revisa.

**Esto es lo que hace escalable la capa B.** Sin él, B2 es un contrato que
nadie cumple en el repo 12.

---

# CAPA C — Xavier (lo que interconecta a todos)

## Por qué aquí y no antes

Xavier tiene ya la superficie correcta (`code find`, `blast-radius`, `call-chain`,
`hubs`, `dependencies`, y un `navigation.rs` de shell). Lo que le falta es el
**dato** que la hace fiable. Si arreglo los docs pero el grafo miente, el
agente acaba confiando en el grafo y pierde más.

## Defectos verificados (con evidencia)

### C1 — `project_root` nunca se escribe → no hay aislamiento

- `Symbol` (code-graph/src/types.rs:209-223) **no tiene** campo `project_root`.
- Los dos `INSERT` (db/mod.rs:574, :641) tampoco.
- `project_root` aparece en el esquema y `ensure_column`
  (db/mod.rs:353, 395-397, 524) — **nunca en un `WHERE`**.
- Medido: `(None, 24833)` → cero símbolos con namespace.
- **Consecuencia medida:** `code_stats` devuelve **496 files / 24833 symbols
  idénticos** para dos repos distintos. El `path` se ignora. Un agente que cite
  esas cifras reporta el repo equivocado sin tener forma de saberlo.

### C2 — `code_scan` borra el índice anterior

`Indexer::index` con `incremental=false` llama **`self.db.clear()`**
(indexer/mod.rs:230) antes de reindexar. Como no hay namespace, "el índice
anterior" es el del *otro* repo.

**Ocurrió dos veces durante esta auditoría, ambas mías:**
1. Un scan de prueba sobrescribió el índice de synapse-trading (14.486 → 472).
2. Un scan CLI con timeout dejó la tabla a 0 y murió a mitad de escritura.

Restaurado ambas veces. Estado actual verificado: **synapse-trading, 24.833
símbolos, 507 ficheros, WAL checkpointado, sano.**

El riesgo no es teórico: cualquier agente que escanee pisará el índice de otro.

### C3 — Dos codebases con comportamientos distintos

| | CLI (`xavier code find/stats`) | HTTP (`/headless/tools/code_find`) |
|---|---|---|
| Aislamiento por repo | sí (`project_id`, `root`) | **no** |
| Índice inexistente | error honesto: *"Ejecuta `xavier code scan .`"* | `count: 0` (se lee como "el símbolo no existe") |
| Auth | **falla con 401 si no hay `XAVIER_TOKEN` en el env** | requiere header |

La CLI es la implementación buena. La HTTP es la que usan los agentes, y es la
que miente. `code_stats` no tiene ni `--path`.

### C4 — Solo 1 de 19 DBs tiene datos

18 `code_graph.db` son stubs de 4 KB con 0 filas. Única con datos:
`apps/xavier/data/code_graph.db` (la de synapse-trading). La tabla `projects`
está **vacía** en todas — `register_project` existe (db/mod.rs:2096) pero nadie
lo llama.

### C5 — Índice saturado de imports

13.6k Variable / 3.2k Import vs 2.4k Function. Un `code_find` any-kind devuelve
ruido; hay que filtrar por kind de definición.

## Plan de trabajo (orden por valor, no por dificultad)

### C-W1. `project_root` de punta a punta — BLOQUEANTE

Sin esto nada más funciona. Todo lo demás es cosmético.

1. Añadir `project_root` a `Symbol`.
2. Escribirlo en los dos `INSERT`.
3. Hacerlo obligatorio en `Indexer::index` (pasa `root`).
4. `register_project(root, name)` al indexar.
5. Migrar el índice actual (reindexar synapse-trading).
6. Test: extender `code-graph/tests/multi_project_test.rs` (ya existe y ya
   comprueba aislamiento — pero sobre filas escritas a mano, no sobre el
   indexer real). El test que falta: **indexar repo A y repo B en la misma DB y
   comprobar que A sigue intacto.** Ese test falla hoy y es la prueba de que C2
   está resuelto.

### C-W2. Parar la destrucción antes de arreglar el aislamiento

Independiente de C-W1 y **más urgente**: no depende de la feature
`code_scan`, añadir un guard: si la DB destino contiene filas con otro
`project_root` y no se pasa `--force`, **rechazar con error explícito** en vez
de borrarlas. Hoy `clear()` no mira nada.

Mientras C-W1 no exista, el guard más barato es: **`xavier code scan` acepta
`--path` explícito y lo exige**, de modo que el repo nunca se deduce por cwd.

### C-W3. `project_root` + `scanned_at` en las respuestas

Convierte un dato ambiguo en auditable. El agente puede decir "esto es de
synapse-trading, indexado el 3 de octubre" en vez de suponer.

### C-W4. Quitar el `count: 0` engañoso

Si no hay índice para ese repo: error con el comando exacto que lo arregla
(que es lo que ya hace la CLI). Un 0 silencioso se lee como "no existe".

### C-W5. Un `xavier nav` para agentes

Aterriza lo que ya existe (`navigation.rs`, `ls/cd/pwd`) en algo que un agente
use sin manual. Hoy nadie lo descubre: 0 usos en las 10 sesiones.

**No es un invento.** Es `navigation.rs` (ls/cd/pwd) que ya está escrita. Le falta entrada.

### C-W6. Reparar el estado

- Reindexar los 17 repos que Workedgruppen tienen stub.
- Decidir explícitamente: ¿grafo global o per-repo? Hoy depende de si
  `workspace == cwd` **por casualidad**, que es la causa raíz de C2.

---

# Qué NO hacer (y por qué)

| No | Razón |
|---|---|
| Arreglar docs de los 125 repos | Sin el generador (B4) no se mantiene. Empezar por los 6 con 4 docs. |
| Mover el grafo a la CLI | La CLI ya aísla; el problema es que la HTTP no usa esa lógica. Reutilizar, no reescribir. |
| `project_root` en `Symbol` sin tocar los INSERT | Da la falsa impresión de aislamiento. |
| Confiar en `code_stats` para reportar | Hoy devuelve el DB del servidor. Verificar con control negativo (dos repos → números distintos). |

---

# Verificación — cómo sé que funcionó

| Tarea | Prueba |
|---|---|
| C-W1 | indexar A y B en una DB; los símbolos de A siguen ahí |
| C-W2 | `code_scan` sobre repo con otro `project_root` → error, no borrado |
| C-W3 | respuesta incluye `project_root` y `scanned_at` |
| C-W4 | repo sin índice → error con el comando, no `count: 0` |
| C-W5 | un agente encuentra un símbolo con `xavier nav` en ≤2 llamadas |
| B3 | test falla si falta un símbolo del top-30 |
| B4 | el mapa generado coincide con `grep` en ≥95% de las filas |

**Métrica de éxito global:** en la próxima auditoría de 10 sesiones,
- build-loops (>5 invocaciones del mismo comando) → **0**
- símbolos buscados con >3 `search_files` → **0**
- citas de `code_stats` sin `project_root` verificable → **0**

---

# Orden de ejecución

1. **Tú:** aprobar skill `0585bb6e` — desbloquea todo lo demás.
2. **Yo:** B2 en los 6 repos de 4 docs (con el mismo formato verificado).
3. **Yo:** B4 generador (convierte B en sustainable).
4. **Xavier:** C-W2 (parar la destrucción) → C-W1 (aislamiento) → C-W3.
5. **Todos:** B3 test de regresión en CI.

C-W2 antes que C-W1 a propósito: mientras el grafo destruya índices, cada test
de la capa C parte de un estado que otro agente puede haber borrado.
