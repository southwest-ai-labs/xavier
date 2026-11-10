/**
 * contentParser.ts
 * Parser inteligente para convertir volcados crudos de memoria (JSONL, transcripts, headers)
 * en información limpia, legible por humanos, con conteo real de menciones y fechas claras.
 */

export interface ParsedMemoryInfo {
  cleanTitle: string;
  cleanPath: string;
  cleanContent: string;
  cleanExcerpt: string;
  matchCount: number;
  lastActiveDate: string;
  createdDate: string;
  sourceType: "conversation" | "code" | "doc" | "fact";
}

/**
 * Cuenta cuántas veces aparece una palabra o frase en el texto (case-insensitive)
 */
export function countKeywordOccurrences(text: string, query: string): number {
  if (!text || !query || !query.trim()) return 0;
  const terms = query
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter((t) => t.length > 0)
    .map((t) => t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));

  if (terms.length === 0) return 0;

  try {
    const regex = new RegExp(`\\b(${terms.join("|")})\\b`, "gi");
    const matches = text.match(regex);
    if (matches && matches.length > 0) return matches.length;

    // Fallback si no hay límite de palabra
    const simpleRegex = new RegExp(`(${terms.join("|")})`, "gi");
    const simpleMatches = text.match(simpleRegex);
    return simpleMatches ? simpleMatches.length : 0;
  } catch {
    return 0;
  }
}

/**
 * Extrae y formatea fechas ISO a formato legible en español
 */
export function formatHumanDate(dateStr?: string): string {
  if (!dateStr) return "Reciente";
  try {
    const date = new Date(dateStr);
    if (isNaN(date.getTime())) return dateStr;

    return new Intl.DateTimeFormat("es-ES", {
      day: "2-digit",
      month: "short",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(date);
  } catch {
    return dateStr;
  }
}

/**
 * Extrae un fragmento de texto (snippet) centrado en la palabra buscada
 */
export function extractKeywordSnippet(
  text: string,
  query: string,
  snippetLength = 220
): string {
  if (!text) return "";
  if (!query || !query.trim()) {
    const trimmed = text.slice(0, snippetLength).trim();
    return trimmed + (text.length > snippetLength ? " ..." : "");
  }

  const terms = query
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter((t) => t.length > 0);

  if (terms.length === 0) {
    const trimmed = text.slice(0, snippetLength).trim();
    return trimmed + (text.length > snippetLength ? " ..." : "");
  }

  const lowerText = text.toLowerCase();
  let firstMatchPos = -1;

  for (const term of terms) {
    const pos = lowerText.indexOf(term);
    if (pos !== -1 && (firstMatchPos === -1 || pos < firstMatchPos)) {
      firstMatchPos = pos;
    }
  }

  if (firstMatchPos === -1) {
    const trimmed = text.slice(0, snippetLength).trim();
    return trimmed + (text.length > snippetLength ? " ..." : "");
  }

  const half = Math.floor(snippetLength / 2);
  let start = Math.max(0, firstMatchPos - half);
  let end = Math.min(text.length, start + snippetLength);

  // Ajustar a los límites de palabras
  if (start > 0) {
    const spaceBefore = text.indexOf(" ", start);
    if (spaceBefore !== -1 && spaceBefore < firstMatchPos) {
      start = spaceBefore + 1;
    }
  }

  if (end < text.length) {
    const spaceAfter = text.lastIndexOf(" ", end);
    if (spaceAfter !== -1 && spaceAfter > firstMatchPos) {
      end = spaceAfter;
    }
  }

  const prefix = start > 0 ? "... " : "";
  const suffix = end < text.length ? " ..." : "";

  return `${prefix}${text.slice(start, end).trim()}${suffix}`;
}

/**
 * Limpia diálogos de usuario y asistente eliminando estructuras JSON y schemas
 */
export function cleanRawTranscript(raw: string): {
  cleanDialogue: string;
  firstUserQuery: string | null;
  lastUpdatedStr: string | null;
  sourceDbStr: string | null;
} {
  if (!raw) {
    return {
      cleanDialogue: "",
      firstUserQuery: null,
      lastUpdatedStr: null,
      sourceDbStr: null,
    };
  }

  let lastUpdatedStr: string | null = null;
  const updateMatch = raw.match(/- \*\*Last Updated\*\*:\s*([^\n\r]+)/);
  if (updateMatch && updateMatch[1]) {
    lastUpdatedStr = updateMatch[1].trim();
  }

  let sourceDbStr: string | null = null;
  const dbMatch = raw.match(/- \*\*Source DB\*\*:\s*([^\n\r]+)/);
  if (dbMatch && dbMatch[1]) {
    sourceDbStr = dbMatch[1].trim();
  }

  let firstUserQuery: string | null = null;
  const dialogueLines: string[] = [];

  // Buscar bloques <user_query> (incluso si están truncados sin tag de cierre)
  const userQueryRegex = /<user_query>\s*([\s\S]*?)(?:<\/user_query>|$)/g;
  let uMatch: RegExpExecArray | null;
  while ((uMatch = userQueryRegex.exec(raw)) !== null) {
    let q = uMatch[1]
      .replace(/<timestamp>.*?<\/timestamp>/gs, "")
      .replace(/^[A-Z\s\(\)]+<\/timestamp>/i, "")
      .replace(/\\n/g, "\n")
      .trim();
    if (q && q.length > 3) {
      if (!firstUserQuery) firstUserQuery = q.split("\n")[0].slice(0, 80);
      dialogueLines.push(`👤 Usuario: ${q}`);
      break;
    }
  }

  // Buscar diálogos con roles JSON
  const roleRegex = /\{"role":"(user|assistant)","message":\{"content":\[\{"type":"text","text":"(.*?)"\}\]/g;
  let rMatch: RegExpExecArray | null;
  while ((rMatch = roleRegex.exec(raw)) !== null) {
    const role = rMatch[1] === "user" ? "👤 Usuario" : "🤖 Asistente";
    let text = rMatch[2]
      .replace(/\\n/g, "\n")
      .replace(/\\"/g, '"')
      .replace(/<timestamp>.*?<\/timestamp>/gs, "")
      .replace(/<user_query>/g, "")
      .replace(/<\/user_query>/g, "")
      .trim();

    // Eliminar comandos shell largos dentro del diálogo para legibilidad humana
    text = text.replace(/```[\s\S]*?```/g, "[bloque de código]");

    if (text) {
      if (!firstUserQuery && rMatch[1] === "user") {
        firstUserQuery = text.split("\n")[0].slice(0, 80);
      }
      dialogueLines.push(`${role}: ${text}`);
    }
  }

  let cleanDialogue = "";
  if (dialogueLines.length > 0) {
    cleanDialogue = dialogueLines.join("\n\n");
  } else {
    // Si no es un formato de chat estructurado, limpiar boilerplate de markdown
    cleanDialogue = raw
      .replace(/^# Agent Conversation Session[\s\S]*?### RAW_JSON_EXTRACT/g, "")
      .replace(/\{"role":.*?\}/g, "")
      .replace(/#[#\s]+/g, "")
      .replace(/\\n/g, "\n")
      .replace(/\\"/g, '"')
      .trim();
  }

  return {
    cleanDialogue,
    firstUserQuery,
    lastUpdatedStr,
    sourceDbStr,
  };
}

/**
 * Analiza y limpia una entrada de memoria RAG completa
 */
export function parseAndCleanMemory(
  rawDoc: any,
  query: string
): ParsedMemoryInfo {
  const rawContent = rawDoc.snippet || rawDoc.memory || rawDoc.content || "";
  const isAgentSession =
    rawContent.includes("Agent Conversation Session") ||
    rawContent.includes("RAW_JSON_EXTRACT") ||
    (typeof rawDoc.user_id === "string" && rawDoc.user_id.includes("agent_memory://"));

  const {
    cleanDialogue,
    firstUserQuery,
    lastUpdatedStr,
    sourceDbStr,
  } = cleanRawTranscript(rawContent);

  const cleanContent = cleanDialogue || rawContent;

  // Extraer ruta real
  let cleanPath = rawDoc.path || "";
  if (sourceDbStr) {
    cleanPath = sourceDbStr;
  } else if (!cleanPath && rawDoc.metadata?.provenance?.file_path) {
    cleanPath = rawDoc.metadata.provenance.file_path;
  } else if (!cleanPath && rawDoc.metadata?.file_path) {
    cleanPath = rawDoc.metadata.file_path;
  } else if (!cleanPath && rawDoc.user_id && rawDoc.user_id.startsWith("agent_memory://")) {
    const parts = rawDoc.user_id.split("/");
    const id = parts[parts.length - 1];
    cleanPath = `transcripts/${id.slice(0, 8)}.jsonl`;
  } else if (cleanPath.startsWith("agent_memory://")) {
    const parts = cleanPath.split("/");
    const id = parts[parts.length - 1];
    cleanPath = `transcripts/${id.slice(0, 8)}.jsonl`;
  }

  if (!cleanPath) {
    cleanPath = rawDoc.id ? `memory://${rawDoc.id}` : "documento_local";
  }

  // Acortar cleanPath para que sea legible y conciso
  if (cleanPath.startsWith("/home/belal/")) {
    cleanPath = cleanPath.replace("/home/belal/", "~/");
  }

  // Generar título conciso para humanos
  let cleanTitle = rawDoc.metadata?.title || rawDoc.title || "";
  const isRawUuid =
    !cleanTitle ||
    /^[0-9a-fA-F-]{20,}$/.test(cleanTitle) ||
    cleanTitle.startsWith("01M") ||
    cleanTitle.includes("JSON_Agent") ||
    cleanTitle.startsWith("Fragmento #") ||
    cleanTitle === "Agent Conversation Session" ||
    cleanTitle === cleanPath;

  if (isRawUuid) {
    if (rawDoc.metadata?.file) {
      cleanTitle = rawDoc.metadata.file;
    } else if (firstUserQuery) {
      cleanTitle = `🗨️ "${firstUserQuery}${firstUserQuery.length >= 70 ? "..." : ""}"`;
    } else if (cleanPath.includes("/")) {
      const parts = cleanPath.split("/");
      cleanTitle = parts[parts.length - 1];
    } else {
      cleanTitle = cleanPath || "Documento";
    }
  }

  // Conteo de menciones: buscar SOLO en el cuerpo y título, ignorando rutas de directorio
  const searchCorpus = `${cleanTitle} ${cleanContent}`;
  const matchCount = countKeywordOccurrences(searchCorpus, query);

  // Snippet contextual enfocado en el diálogo/contenido
  const cleanExcerpt = extractKeywordSnippet(cleanContent, query, 220);

  // Fechas reales
  const rawCreated =
    rawDoc.created_at ||
    rawDoc.metadata?.created_at ||
    rawDoc.metadata?.timestamp;

  const rawLastActive =
    lastUpdatedStr ||
    rawDoc.last_accessed_at ||
    rawDoc.metadata?.last_modified ||
    rawDoc.metadata?.updated_at ||
    rawDoc.metadata?.last_accessed_at ||
    rawCreated;

  let sourceType: ParsedMemoryInfo["sourceType"] = "doc";
  if (isAgentSession) {
    sourceType = "conversation";
  } else if (cleanPath.endsWith(".rs") || cleanPath.endsWith(".ts") || cleanPath.endsWith(".tsx")) {
    sourceType = "code";
  } else if (rawDoc.kind === "fact") {
    sourceType = "fact";
  }

  return {
    cleanTitle,
    cleanPath,
    cleanContent,
    cleanExcerpt,
    matchCount,
    lastActiveDate: formatHumanDate(rawLastActive),
    createdDate: formatHumanDate(rawCreated),
    sourceType,
  };
}
