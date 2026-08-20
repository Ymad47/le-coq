// API client for Le Coq backend
import type { Entity, Relationship, SearchResponse, EntitySummary } from './types';

export async function api<T = any>(path: string, opts: RequestInit = {}): Promise<T> {
  const resp = await fetch(path, opts);
  if (!resp.ok) {
    const text = await resp.text().catch(() => '');
    throw new Error(`${resp.status} ${resp.statusText}${text ? ' — ' + text.slice(0, 200) : ''}`);
  }
  return resp.json();
}

export async function listEntities(limit = 1000): Promise<Entity[]> {
  const data = await api<Entity[] | { entities: Entity[] }>(`/api/v1/entities?limit=${limit}`);
  return Array.isArray(data) ? data : data.entities ?? [];
}

export async function listRelationships(limit = 5000): Promise<Relationship[]> {
  const data = await api<Relationship[] | { relationships: Relationship[] }>(
    `/api/v1/relationships?limit=${limit}`
  );
  return Array.isArray(data) ? data : data.relationships ?? [];
}

export async function smartSearch(
  q: string,
  online: boolean,
  sources?: Set<string>
): Promise<SearchResponse> {
  const params = new URLSearchParams({ q: q.trim(), online: String(online) });
  if (online && sources && sources.size > 0) {
    params.set('sources', Array.from(sources).join(','));
  }
  return api<SearchResponse>(`/api/v1/search?${params}`);
}

export async function getEntitySummary(id: string): Promise<EntitySummary> {
  return api<EntitySummary>(`/api/v1/entities/${encodeURIComponent(id)}/summary`);
}

export async function listSources(): Promise<string[]> {
  return api<string[]>('/api/v1/sources');
}

export async function checkHealth(): Promise<string> {
  return api<string>('/health');
}

export async function reloadPlugins(): Promise<{ reloaded: boolean; loaded: string[]; failed?: any[] }> {
  return api('/api/v1/plugins/reload', { method: 'POST' });
}
