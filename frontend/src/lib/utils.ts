// Shared helpers used across components.
// (Colors live in app.css tokens; see lib/tokens.ts for canvas libraries.)

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
