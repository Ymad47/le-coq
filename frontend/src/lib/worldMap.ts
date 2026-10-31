// Single entry point for loading jsvectormap + its `world_merc` map.
//
// jsvectormap keeps registered maps in an internal variable (Map.maps), not on
// the exported object, so the map data cannot be read back after loading.
// We wrap addMap() once to capture it. Everything that needs the map (Map
// view, home globe) must load it through here: the map file only registers
// itself the first time it is imported.

let pending: Promise<{ jsVectorMap: any; worldMerc: any }> | null = null;

export function loadWorldMerc(): Promise<{ jsVectorMap: any; worldMerc: any }> {
  pending ??= (async () => {
    const mod = await import('jsvectormap');
    const jsVectorMap = (mod as any).default ?? (window as any).jsVectorMap;
    if (!jsVectorMap) throw new Error('jsvectormap non chargé');

    let worldMerc: any = null;
    const addMap = jsVectorMap.addMap;
    jsVectorMap.addMap = (name: string, map: any) => {
      if (name === 'world_merc') worldMerc = map;
      return addMap.call(jsVectorMap, name, map);
    };
    try {
      await import('jsvectormap/dist/maps/world-merc.js');
    } finally {
      jsVectorMap.addMap = addMap;
    }
    if (!worldMerc) throw new Error('world_merc map not registered');
    return { jsVectorMap, worldMerc };
  })();
  // Allow a retry after a failed load (e.g. network error on the chunk)
  pending.catch(() => (pending = null));
  return pending;
}
