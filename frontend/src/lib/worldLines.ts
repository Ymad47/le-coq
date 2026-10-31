// Country outlines as lon/lat lines, rebuilt from the jsvectormap `world_merc`
// map the app already ships (no extra geodata download).
//
// world_merc stores SVG paths in pixel space of a Mercator projection
// (centralMeridian 11.5°E). We invert that projection to get lon/lat, then
// keep the result as *lines*: drawing outlines (not filled polygons) means
// winding order does not matter when re-projecting onto a globe.

import { loadWorldMerc } from './worldMap';

const EARTH_RADIUS = 6378137; // meters, spherical Mercator

export type LonLat = [number, number];

interface MercMap {
  width: number;
  height: number;
  projection?: { centralMeridian?: number };
  insets: { width: number; height: number; left: number; top: number; bbox: { x: number; y: number }[] }[];
  paths: Record<string, { path: string; name?: string }>;
}

/** Splits an SVG path made of M / l / Z commands into absolute pixel rings. */
export function parsePath(d: string): [number, number][][] {
  const rings: [number, number][][] = [];
  let ring: [number, number][] = [];
  let x = 0;
  let y = 0;
  const re = /([MmLlZz])([^MmLlZz]*)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(d))) {
    const cmd = m[1];
    const nums = m[2].split(/[\s,]+/).filter(Boolean).map(Number);
    if (cmd === 'Z' || cmd === 'z') {
      if (ring.length > 1) rings.push([...ring, ring[0]]);
      ring = [];
      continue;
    }
    for (let i = 0; i + 1 < nums.length; i += 2) {
      const relative = cmd === 'l' || cmd === 'm';
      x = relative ? x + nums[i] : nums[i];
      y = relative ? y + nums[i + 1] : nums[i + 1];
      if ((cmd === 'M' || cmd === 'm') && i === 0 && ring.length > 1) {
        rings.push(ring);
        ring = [];
      }
      ring.push([x, y]);
    }
  }
  if (ring.length > 1) rings.push(ring);
  return rings;
}

/** Converts world_merc pixel coordinates back to [longitude, latitude] in degrees. */
export function makeUnprojector(map: MercMap): (p: [number, number]) => LonLat {
  const inset = map.insets[0];
  const [b0, b1] = inset.bbox;
  const sx = (b1.x - b0.x) / inset.width;
  const sy = (b1.y - b0.y) / inset.height;
  const central = map.projection?.centralMeridian ?? 0;
  return ([px, py]) => {
    const mx = b0.x + (px - inset.left) * sx;
    const my = b0.y + (py - inset.top) * sy; // jsvectormap y = -northing
    let lon = (mx / EARTH_RADIUS) * (180 / Math.PI) + central;
    if (lon > 180) lon -= 360;
    if (lon < -180) lon += 360;
    const lat = Math.atan(Math.sinh(-my / EARTH_RADIUS)) * (180 / Math.PI);
    return [lon, lat];
  };
}

/**
 * Splits a ring wherever consecutive points jump across the antimeridian,
 * so the globe does not draw a line straight through the sphere.
 */
function splitAtDateline(ring: LonLat[]): LonLat[][] {
  const parts: LonLat[][] = [];
  let cur: LonLat[] = [];
  for (const p of ring) {
    const prev = cur[cur.length - 1];
    if (prev && Math.abs(p[0] - prev[0]) > 180) {
      if (cur.length > 1) parts.push(cur);
      cur = [];
    }
    cur.push(p);
  }
  if (cur.length > 1) parts.push(cur);
  return parts;
}

/** All country outlines as one GeoJSON MultiLineString (lon/lat degrees). */
export function buildWorldLines(map: MercMap): { type: 'MultiLineString'; coordinates: LonLat[][] } {
  const unproject = makeUnprojector(map);
  const coordinates: LonLat[][] = [];
  for (const { path } of Object.values(map.paths)) {
    for (const ring of parsePath(path)) {
      coordinates.push(...splitAtDateline(ring.map(unproject)));
    }
  }
  return { type: 'MultiLineString', coordinates };
}

let cache: Promise<ReturnType<typeof buildWorldLines>> | null = null;

/** Lazily loads world_merc (same chunk the Map view uses) and converts it once. */
export function loadWorldLines(): Promise<ReturnType<typeof buildWorldLines>> {
  cache ??= (async () => {
    const { worldMerc } = await loadWorldMerc();
    return buildWorldLines(worldMerc as MercMap);
  })();
  return cache;
}
