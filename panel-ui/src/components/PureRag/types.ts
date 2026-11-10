export type ModalityType = "text" | "code" | "image" | "video" | "audio";

export type PureRagSortBy =
  | "term_frequency"        // Más veces nombrada (Frecuencia de término en el documento)
  | "relevance_and_recency" // Relevancia ponderada con recencia de almacenamiento y consulta (por defecto)
  | "similarity_desc"       // Mayor % Similitud estricto
  | "last_accessed"         // Último consultado / tocado / abierto
  | "created_desc";         // Más reciente (fecha almacenamiento)

export interface PureRagItem {
  id: string;
  path: string;
  content: string;
  score: number; // 0.0 - 1.0 (or normalized cosine/RRF)
  similarityPercent: number; // 0.0 - 100.0%
  modality: ModalityType;
  title?: string;
  excerpt?: string;
  cleanExcerpt?: string;
  matchCount?: number;
  rawTimestamp?: number;
  createdAt?: string;
  updatedAt?: string;
  lastAccessedAt?: string;
  author?: string;
  workspaceId?: string;
  project?: string;
  kind?: string;
  clearance?: string;
  metadata?: Record<string, any>;
  // Multimodal fields
  thumbnailUrl?: string;
  mediaUrl?: string;
  spatial?: {
    boundingBox?: [number, number, number, number];
    detectedLabels?: string[];
    ocrText?: string;
  };
  temporal?: {
    startTimeMs: number;
    endTimeMs: number;
    durationMs: number;
    keyframeTimestampMs?: number;
    transcriptSegment?: string;
  };
  // Relational connections for traceability
  connections?: {
    parentId?: string;
    clusterId?: string;
    relatedFiles?: string[];
    tags?: string[];
    symbols?: string[];
  };
}

export interface PureRagFiltersState {
  workspaceId: string;
  project: string;
  userId: string;
  kinds: string[];
  minSimilarity: number; // 0 - 100
  timeHorizon: string; // "all" | "24h" | "7d" | "30d"
  modality: "all" | "text" | "image" | "video";
  sortBy: PureRagSortBy;
}

export const DEFAULT_PURERAG_FILTERS: PureRagFiltersState = {
  workspaceId: "",
  project: "",
  userId: "",
  kinds: [],
  minSimilarity: 0,
  timeHorizon: "all",
  modality: "all",
  sortBy: "relevance_and_recency",
};
