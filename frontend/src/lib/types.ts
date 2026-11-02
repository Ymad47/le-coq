// Shared types for Le Coq frontend

export interface Entity {
  id: string;
  name: string;
  entity_type: string;
  type?: string;  // legacy alias
  properties?: Record<string, any>;
  description?: string | null;
  country?: string | null;
}

export interface Relationship {
  id: string;
  from_entity: string;
  to_entity: string;
  relationship_type: string;
  properties?: Record<string, any>;
  confidence?: number | null;
  sources?: string[] | null;
}

export interface SearchResponse {
  query: string;
  online?: boolean;
  count: number;
  new_items?: number;
  errors?: string[];
  results: Entity[];
}

export interface TopicSearchResponse {
  theme: string;
  entities: Entity[];
  relations: RelationScored[];
  count: number;
  relation_count: number;
}

export interface RelationScored {
  id: string;
  from: string;
  to: string;
  type: string;
  strength: number;
  confidence: number;
  properties?: Record<string, any>;
}

export interface EntitySummary {
  id: string;
  name: string;
  summary: string;
}

export interface MapCountryInfo {
  code: string;
  name: string;
  entities: Entity[];
  count?: number;
}

export interface ArticleRelation {
  from: string;
  to: string;
  relType: string;
  outgoing: boolean;
}

export type View = 'home' | 'list' | 'graph' | 'focus' | 'map' | 'timeline';

/** One dated article on the timeline (GET /api/v1/timeline). */
export interface TimelineArticle {
  id: string;
  title: string;
  url: string;
  domain: string;
  source_country: string;
  language: string;
  /** ISO 8601, UTC. For GDELT: when GDELT first saw the article. */
  published_at: string;
}

export interface TimelineResponse {
  query: string;
  from: string | null;
  to: string | null;
  count: number;
  truncated: boolean;
  ingested: number;
  errors: string[];
  articles: TimelineArticle[];
}

export interface Neighborhood {
  seeds: string[];
  entities: Entity[];
  relationships: Relationship[];
  truncated: boolean;
}

export interface GraphNode {
  id: string;
  label: string;
  type: string;
  degree: number;
  country?: string;
  source?: string;
  description?: string;
  placeholder?: string;
}
