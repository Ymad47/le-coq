// Shared utils — common helpers used across components

export function typeBadge(t: string | undefined | null): string {
  switch ((t ?? '').toLowerCase()) {
    case 'person': return '👤';
    case 'organization': return '🏢';
    case 'location': return '📍';
    case 'event': return '📅';
    case 'asset': return '💼';
    case 'topic': return '🏷️';
    case 'stub': return '⏳';
    default: return '📦';
  }
}

export function stableRandom(str: string): number {
  let h = 2166136261;
  for (let i = 0; i < str.length; i++) {
    h ^= str.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return (h >>> 0) / 4294967296;
}

export function hslToHex(h: number, s: number, l: number): string {
  const a = (s * Math.min(l, 1 - l)) / 1;
  const f = (n: number) => {
    const k = (n + h / 30) % 12;
    const color = l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
    return Math.round(255 * color).toString(16).padStart(2, '0');
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}

export function colorForType(t: string | undefined | null): string {
  const base = (t ?? '').toLowerCase();
  switch (base) {
    case 'person': return '#f97316';
    case 'organization': return '#3b82f6';
    case 'location': return '#10b981';
    case 'event': return '#a855f7';
    case 'asset': return '#eab308';
    case 'stub': return '#6b7280';
    case 'topic': return '#2dd4bf';
    default: return '#94a3b8';
  }
}

export function colorForEdge(rel: string | undefined | null): string {
  const r = (rel ?? '').toLowerCase();
  if (r === 'mentions' || r === 'related' || r === 'related_to' || r === 'related topic') {
    return 'rgba(148, 163, 184, 0.3)';
  }
  return 'rgba(45, 212, 191, 0.5)';
}

export function extractYear(props: Record<string, any> | undefined): number | null {
  if (!props) return null;
  for (const k of ['date', 'year', 'founded', 'established', 'born', 'published', 'first_seen']) {
    const v = props[k];
    if (typeof v === 'string' && v) {
      const m = v.match(/\b(1[5-9]\d{2}|20\d{2})\b/);
      if (m) return parseInt(m[1], 10);
    }
    if (typeof v === 'number' && v > 1500 && v < 2100) return v;
  }
  return null;
}

export function stripSourcePrefix(id: string): string {
  return id
    .replace(/^wiki_|^wd_item_|^uncomtrade_country_|^eurostat_country_/, '')
    .replace(/_/g, ' ');
}
