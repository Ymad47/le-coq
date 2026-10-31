// Canvas-based libraries (cytoscape, jsvectormap, the globe) cannot use CSS
// variables directly. They read the current theme's values through here, so
// colors stay defined once, in app.css.

/** Current value of a CSS custom property on <html>, e.g. cssVar('--viz-edge'). */
export function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

/** Linear mix of two #rrggbb colors; t in [0, 1]. */
export function mixHex(a: string, b: string, t: number): string {
  const pa = parseHex(a);
  const pb = parseHex(b);
  if (!pa || !pb) return b;
  const k = Math.min(1, Math.max(0, t));
  const c = pa.map((v, i) => Math.round(v + (pb[i] - v) * k));
  return `#${c.map((v) => v.toString(16).padStart(2, '0')).join('')}`;
}

function parseHex(h: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(h.trim());
  if (!m) return null;
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Snapshot of the data-viz palette for the active theme. */
export function vizPalette() {
  return {
    text: cssVar('--text'),
    text2: cssVar('--text-2'),
    surface: cssVar('--surface'),
    brand: cssVar('--brand'),
    brandText: cssVar('--brand-text'),
    node: cssVar('--viz-node'),
    nodeHub: cssVar('--viz-node-hub'),
    nodeStub: cssVar('--viz-node-stub'),
    edge: cssVar('--viz-edge'),
    edgeStrong: cssVar('--viz-edge-strong'),
    label: cssVar('--viz-label'),
    mapEmpty: cssVar('--viz-map-empty'),
    mapLow: cssVar('--viz-map-low'),
    mapHigh: cssVar('--viz-map-high'),
    mapHover: cssVar('--viz-map-hover'),
    mapStroke: cssVar('--viz-map-stroke'),
    globeSea: cssVar('--viz-globe-sea'),
    globeLand: cssVar('--viz-globe-land'),
    globeGrid: cssVar('--viz-globe-grid'),
    globeArc: cssVar('--viz-globe-arc'),
    globeCity: cssVar('--viz-globe-city'),
    globeRim: cssVar('--viz-globe-rim'),
  };
}
