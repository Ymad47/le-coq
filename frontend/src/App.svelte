<script>
  import { onMount, tick } from 'svelte';
  import cytoscape from 'cytoscape';

  let entities = [];
  let relationships = [];
  let query = '';
  let loading = false;
  let error = '';
  let view = 'list'; // 'list' | 'graph' | 'focus' | 'map'
  let focusHubId = null;
  let health = '';

  let ingestOnline = true; // online par défaut — UX souveraine
  let newItems = 0;
  let searchErrors = [];
  let searching = false;
  let allSources = [];
  let selectedSources = new Set();

  // Sources rapides et stables en smart default
  const DEFAULT_SOURCES = ['wikipedia', 'wikidata', 'gleif', 'eurostat'];

  /** @type {HTMLDivElement} */
  let graphEl;
  /** @type {import('cytoscape').Core | null} */
  let cy = null;
  /** @type {HTMLDivElement} */
  let mapEl;
  /** @type {any} */
  let worldMap = null;
  /** @type {{code: string, name: string, entities: any[], count?: number} | null} */
  let mapInfo = null;

  async function api(path, opts = {}) {
    const resp = await fetch(path, opts);
    if (!resp.ok) {
      const text = await resp.text().catch(() => '');
      throw new Error(`${resp.status} ${resp.statusText}${text ? ' — ' + text.slice(0, 200) : ''}`);
    }
    return resp.json();
  }

  async function search() {
    if (!query.trim()) {
      // Pas de query → juste lister DB
      loading = true;
      try {
        const data = await api('/api/v1/entities?limit=1000');
        entities = Array.isArray(data) ? data : data.entities ?? [];
        newItems = 0;
        searchErrors = [];
      } catch (e) {
        error = `Chargement échoué : ${e.message}`;
      }
      loading = false;
      return;
    }

    loading = true;
    searching = true;
    error = '';
    newItems = 0;
    searchErrors = [];

    try {
      const params = new URLSearchParams({ q: query.trim(), online: ingestOnline });
      if (ingestOnline && selectedSources.size > 0) {
        params.set('sources', Array.from(selectedSources).join(','));
      }
      const data = await api(`/api/v1/search?${params}`);
      const results = data.results ?? [];
      // Normaliser pour l'UI
      entities = results.map((r) => ({
        id: r.id,
        name: r.name,
        entity_type: r.entity_type,
        properties: r.properties,
        description: r.properties?.description ?? null,
        country: r.properties?.country ?? null,
      }));
      newItems = data.new_items ?? 0;
      searchErrors = data.errors ?? [];
    } catch (e) {
      error = `Recherche échouée : ${e.message}`;
    }
    loading = false;
    searching = false;
  }

  async function loadGraph() {
    loading = true;
    error = '';
    try {
      const [ents, rels] = await Promise.all([
        api('/api/v1/entities?limit=1000'),
        api('/api/v1/relationships?limit=5000'),
      ]);
      entities = Array.isArray(ents) ? ents : ents.entities ?? [];
      relationships = Array.isArray(rels) ? rels : rels.relationships ?? [];
      graphLayout = 'cose';
      focusHubId = null;
      view = 'graph';
      await tick();
      renderGraph();
      
      // Auto-center on query match if searching (after layout finishes)
      if (query.trim() && cy) {
        const qLower = query.trim().toLowerCase();
        const match = entities.find((e) => (e.name ?? '').toLowerCase() === qLower)
          ?? entities.find((e) => (e.name ?? '').toLowerCase().startsWith(qLower));
        if (match) {
          const doCenter = () => {
            const node = cy.getElementById(match.id);
            if (node && node.length) {
              cy.animate({ center: { eles: node }, duration: 300 });
              node.select();
              selectedNode = node.data();
              const neighborhood = node.closedNeighborhood();
              cy.elements().removeClass('dim');
              cy.elements().not(neighborhood).addClass('dim');
            }
          };
          cy.one('layoutstop', doCenter);
          setTimeout(() => { if (cy && !selectedNode) doCenter(); }, 800);
        }
      }
    } catch (e) {
      error = `Chargement du graphe échoué : ${e.message}`;
    }
    loading = false;
  }

  // Focus: graph-centric view centered on the best-matching node for current query
  // skipSearch=true → skip online fetch (entity already in DB, e.g. opened from drawer/article)
  // directHubId → exact entity id to center on (skips name matching entirely)
  async function loadFocus(skipSearch = false, directHubId = null) {
    if (!query.trim() && !directHubId) {
      error = 'Entrez une recherche d\'abord pour utiliser Vue Focus.';
      return;
    }
    loading = true;
    searching = !skipSearch;
    error = '';
    newItems = 0;
    searchErrors = [];

    try {
      // 1) Smart search (skippable — useful when we know the entity exists)
      const qLower = query.trim().toLowerCase();
      let hub = null;
      if (!skipSearch) {
        const params = new URLSearchParams({ q: query.trim(), online: ingestOnline });
        if (ingestOnline && selectedSources.size > 0) {
          params.set('sources', Array.from(selectedSources).join(','));
        }
        const data = await api(`/api/v1/search?${params}`);
        const results = data.results ?? [];
        newItems = data.new_items ?? 0;
        searchErrors = data.errors ?? [];
        hub = results.find((r) => (r.name ?? '').toLowerCase() === qLower)
          ?? results.find((r) => (r.name ?? '').toLowerCase().startsWith(qLower))
          ?? results[0];
      }

      // 2) Load full graph for visualization
      const [ents, rels] = await Promise.all([
        api('/api/v1/entities?limit=1000'),
        api('/api/v1/relationships?limit=5000'),
      ]);
      entities = Array.isArray(ents) ? ents : ents.entities ?? [];
      relationships = Array.isArray(rels) ? rels : rels.relationships ?? [];

      // 3) Hub: direct ID wins, else entity match, else graph entity
      if (directHubId) {
        focusHubId = directHubId;
      } else {
        if (!hub) {
          hub = entities.find((e) => (e.name ?? '').toLowerCase() === qLower)
            ?? entities.find((e) => (e.name ?? '').toLowerCase().startsWith(qLower))
            ?? entities.find((e) => (e.name ?? '').toLowerCase().includes(qLower));
        }
        focusHubId = hub?.id ?? null;
      }

      // 4) Layout: concentric centered on hub
      graphLayout = 'concentric';
      view = 'focus';
      await tick();
      renderGraph();

      // 5) Wait for layout to finish, then center/select hub node
      if (focusHubId && cy) {
        const centerHub = () => {
          const node = cy.getElementById(focusHubId);
          if (node && node.length) {
            selectedNode = node.data();
            cy.animate({
              center: { eles: node },
              zoom: { level: 1.2, position: node.position() },
              duration: 300,
            });
            node.select();
            const neighborhood = node.closedNeighborhood();
            cy.elements().removeClass('dim');
            cy.elements().not(neighborhood).addClass('dim');
          } else {
            // Hub not found as node — try substring match on label
            const alt = cy.nodes().filter((n) =>
              (n.data('label') ?? '').toLowerCase().includes(qLower)
            ).first();
            if (alt && alt.length) {
              focusHubId = alt.id();
              centerHub();
            }
          }
        };
        // layoutstop fires after cose/concentric finishes
        cy.one('layoutstop', centerHub);
        // Safety fallback if layoutstop never fires
        setTimeout(() => { if (cy && !selectedNode) centerHub(); }, 800);
      }
    } catch (e) {
      error = `Vue Focus échouée : ${e.message}`;
    }
    loading = false;
    searching = false;
  }

  // Country name → ISO3 code (aligned with jsvectormap world_merc paths)
  const NAME_TO_ISO = {
    'France': 'FRA', 'United States': 'USA', 'United States of America': 'USA',
    'China': 'CHN', 'Germany': 'DEU', 'United Kingdom': 'GBR', 'UK': 'GBR',
    'Russia': 'RUS', 'Brazil': 'BRA', 'India': 'IND', 'Japan': 'JPN',
    'South Africa': 'ZAF', 'Canada': 'CAN', 'Mexico': 'MEX', 'Australia': 'AUS',
    'South Korea': 'KOR', 'Korea': 'KOR',
    'Spain': 'ESP', 'Italy': 'ITA', 'Belgium': 'BEL', 'Netherlands': 'NLD',
    'Poland': 'POL', 'Turkey': 'TUR', 'Saudi Arabia': 'SAU',
    'United Arab Emirates': 'ARE', 'Qatar': 'QAT',
    'Egypt': 'EGY', 'Morocco': 'MAR', 'Algeria': 'DZA', 'Nigeria': 'NGA',
    'Kenya': 'KEN', 'Ghana': 'GHA', 'Ethiopia': 'ETH',
    'Switzerland': 'CHE', 'Austria': 'AUT', 'Denmark': 'DNK', 'Finland': 'FIN',
    'Greece': 'GRC', 'Ireland': 'IRL', 'Norway': 'NOR', 'Portugal': 'PRT',
    'Sweden': 'SWE', 'Czech Republic': 'CZE', 'Hungary': 'HUN', 'Romania': 'ROU',
    'Bulgaria': 'BGR', 'Croatia': 'HRV', 'Slovakia': 'SVK', 'Slovenia': 'SVN',
    'Estonia': 'EST', 'Latvia': 'LVA', 'Lithuania': 'LTU', 'Ukraine': 'UKR',
    'Israel': 'ISR', 'Iran': 'IRN', 'Iraq': 'IRQ', 'Jordan': 'JOR',
    'Lebanon': 'LBN', 'Syria': 'SYR', 'Yemen': 'YEM', 'Oman': 'OMN',
    'Bahrain': 'BHR', 'Kuwait': 'KWT', 'New Zealand': 'NZL',
    'Argentina': 'ARG', 'Chile': 'CHL', 'Colombia': 'COL', 'Peru': 'PER',
    'Venezuela': 'VEN', 'Pakistan': 'PAK', 'Bangladesh': 'BGD',
    'Philippines': 'PHL', 'Indonesia': 'IDN', 'Malaysia': 'MYS',
    'Singapore': 'SGP', 'Thailand': 'THA', 'Vietnam': 'VNM',
    'Taiwan': 'TWN', 'Hong Kong': 'HKG',
  };

  // Map view: world choropleth — countries colored by entity count
  async function loadMap() {
    loading = true;
    error = '';
    view = 'map';
    try {
      const ents = await api('/api/v1/entities?limit=1000');
      const list = Array.isArray(ents) ? ents : ents.entities ?? [];
      entities = list;

      // Count entities per ISO country code
      const counts = new Map();
      for (const e of list) {
        let iso = e.properties?.iso3 ?? e.properties?.iso3_code;
        if (!iso && e.id?.startsWith('uncomtrade_country_')) {
          // Name in ID — map through NAME_TO_ISO
          const name = e.id.slice('uncomtrade_country_'.length).replace(/_/g, ' ');
          iso = NAME_TO_ISO[name];
        }
        if (!iso && e.id?.startsWith('eurostat_country_')) {
          // FR, DE etc (2-letter)
          const two = e.id.slice('eurostat_country_'.length).split('_')[0]?.toUpperCase();
          const ISO2_TO_3 = { FR: 'FRA', DE: 'DEU', IT: 'ITA', ES: 'ESP', GB: 'GBR', NL: 'NLD', BE: 'BEL', PL: 'POL', PT: 'PRT', AT: 'AUT', CH: 'CHE', SE: 'SWE', NO: 'NOR', DK: 'DNK', FI: 'FIN', GR: 'GRC', IE: 'IRL', RO: 'ROU', CZ: 'CZE', HU: 'HUN', SK: 'SVK', SI: 'SVN', BG: 'BGR', HR: 'HRV', EE: 'EST', LV: 'LVA', LT: 'LTU', LU: 'LUX', MT: 'MLT', CY: 'CYP' };
          iso = ISO2_TO_3[two];
        }
        if (!iso || !/^[A-Z]{3}$/.test(iso)) continue;
        counts.set(iso, (counts.get(iso) ?? 0) + 1);
      }

      console.log('[map] country counts:', Object.fromEntries(counts));
      await tick();
      renderMap(counts);
    } catch (e) {
      error = `Vue carte échouée : ${e.message}`;
    }
    loading = false;
  }

  async function renderMap(counts) {
    if (!mapEl) return;
    try {
      // Lazy-load jsvectormap & world map once
      const mod = await import('jsvectormap');
      await import('jsvectormap/dist/maps/world-merc.js');
      // jsvectormap reads window.jsVectorMap
      const jsVectorMap = mod.default ?? window.jsVectorMap;
      if (!jsVectorMap) { error = 'jsvectormap non chargé'; return; }
      worldMap?.destroy?.();
      const values = Object.fromEntries(counts);
      const max = Math.max(...counts.values(), 1);

      // Track the region names locally (independent from jsvectormap internals)
      const regionNames = new Map();

      worldMap = new jsVectorMap({
        selector: mapEl,
        map: 'world_merc',
        backgroundColor: 'transparent',
        regionStyle: {
          initial: { fill: 'rgba(255,255,255,0.06)', stroke: 'rgba(255,255,255,0.12)', strokeWidth: 0.4 },
          hover: { fill: '#14b8a6', fillOpacity: 0.75, stroke: 'rgba(255,255,255,0.3)' },
          selected: { fill: '#2dd4bf', fillOpacity: 0.95 },
        },
        series: {
          regions: [{
            values,
            scale: ['#1f2937', '#2dd4bf'],
            normalizeFunction: 'linear',
            min: 0,
            max,
          }],
        },
        onRegionTooltipShow: (evt, tooltip, code) => {
          const n = counts.get(code) ?? 0;
          const name = regionNames.get(code) ?? code;
          tooltip.text(`${name} — ${n} entité${n !== 1 ? 's' : ''}`, true);
        },
        onRegionSelected: (evt, code, isSelected, selectedRegions) => {
          if (!isSelected) return;
          const n = counts.get(code) ?? 0;
          const name = regionNames.get(code) ?? code;
          if (n === 0) {
            // No entities — show info but don't search
            mapInfo = { code, name, entities: [] };
            return;
          }
          // Open country detail panel with its entities
          const countryEntities = entities.filter((e) => {
            let iso = e.properties?.iso3 ?? e.properties?.iso3_code;
            if (!iso && e.id?.startsWith('uncomtrade_country_')) {
              const eName = e.id.slice('uncomtrade_country_'.length).replace(/_/g, ' ');
              iso = NAME_TO_ISO[eName];
            }
            if (!iso && e.id?.startsWith('eurostat_country_')) {
              const two = e.id.slice('eurostat_country_'.length).split('_')[0]?.toUpperCase();
              const ISO2_TO_3 = { FR: 'FRA', DE: 'DEU', IT: 'ITA', ES: 'ESP', GB: 'GBR', NL: 'NLD', BE: 'BEL', PL: 'POL', PT: 'PRT', AT: 'AUT', CH: 'CHE', SE: 'SWE', NO: 'NOR', DK: 'DNK', FI: 'FIN', GR: 'GRC', IE: 'IRL', RO: 'ROU', CZ: 'CZE', HU: 'HUN', SK: 'SVK', SI: 'SVN', BG: 'BGR', HR: 'HRV', EE: 'EST', LV: 'LVA', LT: 'LTU', LU: 'LUX', MT: 'MLT', CY: 'CYP' };
              iso = ISO2_TO_3[two];
            }
            return iso === code;
          });
          mapInfo = { code, name, entities: countryEntities, count: n };
        },
      });

      // Pre-populate region names from the map data itself
      try {
        const paths = jsVectorMap.maps?.world_merc?.paths ?? {};
        for (const [code, data] of Object.entries(paths)) {
          if (data?.name) regionNames.set(code, data.name);
        }
      } catch (e) {
        console.warn('[map] could not preload region names', e);
      }
    } catch (e) {
      error = `Render map: ${e.message}`;
    }
  }

  // Color palette by entity type (cohérent, lisible)
  const TYPE_COLORS = {
    person: '#c9a227',       // doré (brand)
    organization: '#4299e1', // bleu
    country: '#48bb78',      // vert
    location: '#ed8936',     // orange
    event: '#f56565',        // rouge
    indicator: '#9f7aea',    // violet
    topic: '#38b2ac',        // teal (wikipedia topics)
    stub: '#4a5568',         // stub auto-créé (endpoint de relation)
    unknown: '#718096',      // gris
  };

  const EDGE_COLORS = {
    parent_of: '#2dd4bf',
    exports_to: '#38bdf8',
    imports_from: '#38bdf8',
    has_indicator: 'rgba(255,255,255,0.2)',
    default: 'rgba(255,255,255,0.15)',
  };

  // Cluster coloring: each top hub gets a stable random hue (seeded by ID),
  // its neighborhood follows in a lightness gradient by BFS distance.
  function stableRandom(str) {
    // FNV-1a hash -> 0..1: same id always yields same color across reloads
    let h = 2166136261;
    for (let i = 0; i < str.length; i++) {
      h ^= str.charCodeAt(i);
      h = Math.imul(h, 16777619);
    }
    return ((h >>> 0) % 1000) / 1000;
  }

  function hslToHex(h, s, l) {
    const c = (1 - Math.abs(2 * l - 1)) * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = l - c / 2;
    let r, g, b;
    if (h < 60) { r = c; g = x; b = 0; }
    else if (h < 120) { r = x; g = c; b = 0; }
    else if (h < 180) { r = 0; g = c; b = x; }
    else if (h < 240) { r = 0; g = x; b = c; }
    else if (h < 300) { r = x; g = 0; b = c; }
    else { r = c; g = 0; b = x; }
    const c2 = (v) => Math.round((v + m) * 255).toString(16).padStart(2, '0');
    return `#${c2(r)}${c2(g)}${c2(b)}`;
  }

  // Assign each node a color: hubs own their hue, children inherit with distance fade
  function assignClusterColors(nodes, edges, deg, maxDegree, topRatio = 0.08) {
    // Hubs = top-degree nodes (~top 8%, min 3)
    const sorted = [...nodes].sort((a, b) => (deg.get(b.data.id) ?? 0) - (deg.get(a.data.id) ?? 0));
    const hubCount = Math.max(3, Math.ceil(nodes.length * topRatio));
    const hubs = sorted.filter((n) => (deg.get(n.data.id) ?? 0) > 0).slice(0, hubCount);

    const color = new Map(); // nodeId -> hex
    const hubHue = new Map(); // hubId -> hue

    // Each hub: hue from seeded hash (spread with golden angle for variety)
    hubs.forEach((hub, i) => {
      const base = stableRandom(hub.data.id) * 360;
      const hue = Math.round((base + i * 137.508) % 360); // golden angle spread
      hubHue.set(hub.data.id, hue);
      color.set(hub.data.id, hslToHex(hue, 0.75, 0.55)); // hubs: vivid
    });

    // BFS from all hubs simultaneously: nearest hub wins, deeper = lighter/duller
    const adj = new Map(); // id -> [neighborId]
    const push = (a, b) => { if (!adj.has(a)) adj.set(a, []); adj.get(a).push(b); };
    edges.forEach((e) => {
      push(e.data.source, e.data.target);
      push(e.data.target, e.data.source); // undirected for clustering
    });

    const queue = hubs.map((h) => ({ id: h.data.id, depth: 0, hue: hubHue.get(h.data.id) }));
    const visited = new Set(hubs.map((h) => h.data.id));
    while (queue.length) {
      const { id, depth, hue } = queue.shift();
      const dist = depth; // 1, 2, 3...
      color.set(id, hslToHex(hue, Math.max(0.25, 0.7 - depth * 0.14), Math.max(0.35, 0.62 - depth * 0.07)));
      for (const nb of adj.get(id) ?? []) {
        if (!visited.has(nb)) {
          visited.add(nb);
          queue.push({ id: nb, depth: depth + 1, hue });
        }
      }
    }

    // Unreachable leftovers (isolated/disconnected from hubs): slate
    nodes.forEach((n) => {
      if (!color.has(n.data.id)) color.set(n.data.id, '#2d3748');
    });

    return { color, hubIds: new Set(hubs.map((h) => h.data.id)) };
  }

  let selectedNode = null;
  let articleEntity = null; // entity open in article drawer (list view click)
  let articleRelations = [];
  let articleSummary = null; // narrative summary from /entities/:id/summary
  let graphLayout = 'cose';
  let nodeCount = 0;
  let edgeCount = 0;
  let showIsolated = false; // hide degree-0 nodes by default
  let hideWeakLinks = true; // hide 'related_to' edges pointing to stubs
  let maxYear = new Date().getFullYear(); // temporal filter: show only items <= this year
  const MIN_YEAR = 2000;

  // Cache resolved Wikidata labels (Q142 -> "France")
  const qidLabelCache = new Map();

  // Resolve Wikidata Q-codes to human labels via wbgetentities (batch, no key needed)
  async function resolveQidLabels(ids) {
    const unknown = ids.filter((id) => id.startsWith('wd_item_') && !qidLabelCache.has(id));
    if (!unknown.length) return;
    const BATCH = 50; // MediaWiki API limit
    for (let i = 0; i < unknown.length; i += BATCH) {
      const chunk = unknown.slice(i, i + BATCH);
      const qids = chunk.map((id) => id.slice(8)).join('|'); // strip "wd_item_"
      try {
        const url = `https://www.wikidata.org/w/api.php?action=wbgetentities&ids=${qids}&props=labels&languages=fr|en&format=json&origin=*`;
        const resp = await fetch(url);
        const data = await resp.json();
        for (const id of chunk) {
          const qid = id.slice(8);
          const ent = data.entities?.[qid];
          const label = ent?.labels?.fr?.value ?? ent?.labels?.en?.value;
          if (label) qidLabelCache.set(id, label);
        }
      } catch {
        // network fail -> keep Q-code labels
      }
    }
  }

  function colorForType(t) {
    if (!t) return TYPE_COLORS.unknown;
    return TYPE_COLORS[t.toLowerCase()] ?? TYPE_COLORS.unknown;
  }

  function colorForEdge(rel) {
    if (!rel) return EDGE_COLORS.default;
    return EDGE_COLORS[rel.toLowerCase()] ?? EDGE_COLORS.default;
  }

  // Extract year from properties (multiple possible fields)
  function extractYear(props) {
    if (!props) return null;
    for (const key of ['year', 'period', 'refYear', 'date']) {
      const v = props[key];
      if (!v) continue;
      const n = parseInt(v, 10);
      if (!isNaN(n) && n >= 1900 && n <= 2100) return n;
      // Try ISO date string 2024-03-15
      if (typeof v === 'string') {
        const m = v.match(/\b(19|20)\d{2}\b/);
        if (m) { const y = parseInt(m[0], 10); return y >= 1900 && y <= 2100 ? y : null; }
      }
    }
    return null;
  }

  async function renderGraph() {
    if (!graphEl) return;

    // Resolve Q-code labels for referenced Wikidata items before building nodes
    const referencedIds = new Set();
    relationships.forEach((r) => {
      const f = r.source_id ?? r.from_entity;
      const t = r.target_id ?? r.to_entity;
      if (f) referencedIds.add(f);
      if (t) referencedIds.add(t);
    });
    const nodeMap = new Map(entities.map((e) => [e.id, e]));
    const placeholderIds = [...referencedIds].filter((id) => !nodeMap.has(id));
    await resolveQidLabels(placeholderIds);

    cy?.destroy();

    // Start with known entities
    const nodes = entities.map((e) => ({
      data: {
        id: e.id,
        label: e.name,
        type: e.entity_type ?? e.type ?? 'unknown',
        description: e.description ?? e.properties?.description ?? '',
        country: e.country ?? e.properties?.country ?? '',
        source: e.properties?.source ?? '',
      },
    }));

    // Add placeholder nodes for referenced but missing entities
    placeholderIds.forEach((id) => {
      // Priority: resolved Wikidata label > prefix-stripped fallback > raw id
      let label = qidLabelCache.get(id);
      if (!label) {
        if (id.startsWith('wiki_')) label = id.slice(5).replace(/_/g, ' ');
        else if (id.startsWith('wd_item_')) label = id.slice(8);
        else if (id.startsWith('eurostat_country_')) label = id.slice(17).replace(/_/g, ' ');
        else label = id;
      }
      label = (label || id).trim() || id;
      nodes.push({
        data: {
          id,
          label,
          type: 'unknown',
          description: '',
          country: '',
          source: '',
          placeholder: 'true',
        },
      });
    });

    // Edges: accept both old and new keys, filter weak links if enabled, then temporal filter
    const edges = relationships
      .map((r) => {
        const from = r.source_id ?? r.from_entity;
        const to = r.target_id ?? r.to_entity;
        const relType = r.relation_type ?? r.relationship_type ?? r.type ?? '';
        const year = extractYear(r.properties);
        return { from, to, relType, rawId: r.id, year };
      })
      .filter((e) => e.from && e.to)
      // Temporal filter: drop edges dated after maxYear
      .filter((e) => e.year === null || e.year <= maxYear)
      .map((e) => ({
        data: {
          id: e.rawId,
          source: e.from,
          target: e.to,
          label: e.relType,
          relType: e.relType,
          year: e.year,
        },
      }));

    // Build set of stub node IDs (placeholder nodes or stub-typed entities)
    const stubIds = new Set(
      nodes
        .filter((n) => n.data.placeholder === 'true' || n.data.type === 'stub')
        .map((n) => n.data.id)
    );

    // Filter weak links: 'related_to' edges pointing to stubs.
    // Strong edges (organization, location, participants...) always kept.
    const WEAK_REL_TYPES = new Set(['related', 'related_to', 'related topic', 'mentions']);
    const filteredEdges = hideWeakLinks
      ? edges.filter((e) => {
          const isWeak = WEAK_REL_TYPES.has((e.data.relType ?? '').toLowerCase());
          const touchesStub = stubIds.has(e.data.target) || stubIds.has(e.data.source);
          return !(isWeak && touchesStub);
        })
      : edges;

    // Keep stub nodes that lost all their edges from being orphaned:
    // if a node is only connected via filtered-out edges, drop it too (unless focus hub)
    const keptIds = new Set();
    filteredEdges.forEach((e) => { keptIds.add(e.data.source); keptIds.add(e.data.target); });

    // Compute degree from FILTERED edges (weak links don't count)
    const deg = new Map();
    filteredEdges.forEach((e) => {
      deg.set(e.data.source, (deg.get(e.data.source) ?? 0) + 1);
      deg.set(e.data.target, (deg.get(e.data.target) ?? 0) + 1);
    });

    // Find max degree for heat map scaling
    const maxDegree = Math.max(...deg.values(), 0);

    // Filter isolated nodes (degree 0 in filtered graph) unless explicitly shown
    const visibleNodes = showIsolated
      ? nodes
      : nodes.filter((n) => (deg.get(n.data.id) ?? 0) > 0 || n.data.id === focusHubId);

    // Cluster colors based on filtered edges
    const { color: clusterColor, hubIds } = assignClusterColors(visibleNodes, filteredEdges, deg, maxDegree);

    visibleNodes.forEach((n) => {
      const d = deg.get(n.data.id) ?? 0;
      n.data.degree = d;
      // Quadratic sizing: hubs pop much more than leaves
      const ratio = maxDegree > 0 ? d / maxDegree : 0;
      n.data.size = Math.min(24 + 46 * ratio * ratio, 70);
      n.data.color = clusterColor.get(n.data.id);
      n.data.hot = hubIds.has(n.data.id) ? 'true' : 'false';
    });

    nodeCount = visibleNodes.length;
    edgeCount = filteredEdges.length;
    const hiddenCount = nodes.length - visibleNodes.length;

    const showEdgeLabel = edgeCount < 30;

    const layoutOpts = {
      name: graphLayout,
      animate: false,
      padding: 30,
      ...(graphLayout === 'concentric' && {
        minNodeSpacing: 30,
        avoidOverlap: true,
        concentric: (node) => node.data('degree') ?? 0,
        levelWidth: () => 1,
      }),
      ...(graphLayout === 'breadthfirst' && {
        directed: true,
        spacingFactor: 1.2,
        avoidOverlap: true,
      }),
    };

    cy = cytoscape({
      container: graphEl,
      elements: [...visibleNodes, ...filteredEdges],
      style: [
        {
          selector: 'node',
          style: {
            label: 'data(label)',
            'background-color': (ele) => {
              // Use heat map color if available (degree-based), otherwise type-based
              return ele.data('color') ?? colorForType(ele.data('type'));
            },
            color: 'rgba(255,255,255,0.9)',
            'font-size': 10,
            'font-weight': 500,
            'text-valign': 'bottom',
            'text-margin-y': 4,
            'text-wrap': 'ellipsis',
            'text-max-width': '80px',
            width: 'data(size)',
            height: 'data(size)',
            'border-width': 0,
            'border-color': 'rgba(255,255,255,0.25)',
          },
        },
        {
          selector: 'node[placeholder = "true"]',
          style: {
            'background-color': '#4a5568',
            'border-width': 2,
            'border-style': 'dashed',
            'border-color': '#718096',
            opacity: 0.7,
          },
        },
        {
          // Hot hubs: subtle gold halo to signal importance
          selector: 'node[hot = "true"]',
          style: {
            'border-width': 3,
            'border-color': '#2dd4bf',
            'border-opacity': 0.9,
          },
        },
        {
          selector: 'node:selected',
          style: {
            'border-width': 3,
            'border-color': 'rgba(255,255,255,0.25)',
            'overlay-color': '#2dd4bf',
            'overlay-opacity': 0.15,
          },
        },
        {
          selector: 'node:active',
          style: {
            'overlay-color': '#2dd4bf',
            'overlay-opacity': 0.25,
          },
        },
        {
          selector: 'edge',
          style: {
            label: showEdgeLabel ? 'data(label)' : '',
            'line-color': (ele) => colorForEdge(ele.data('relType')),
            'target-arrow-color': (ele) => colorForEdge(ele.data('relType')),
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            'font-size': 8,
            color: 'rgba(255,255,255,0.5)',
            width: (ele) => (ele.data('relType') === 'parent_of' ? 2.5 : 1.5),
            opacity: 0.85,
          },
        },
        {
          selector: 'edge:selected',
          style: {
            width: 4,
            opacity: 1,
          },
        },
      ],
      layout: layoutOpts,
    });

    // Click node → show details panel
    cy.on('tap', 'node', (evt) => {
      const d = evt.target.data();
      selectedNode = d;
      cy.elements().removeClass('dim');
      // Dim non-neighbors
      const neighborhood = evt.target.closedNeighborhood();
      cy.elements().not(neighborhood).addClass('dim');
    });

    // Click background → reset
    cy.on('tap', (evt) => {
      if (evt.target === cy) {
        selectedNode = null;
        cy.elements().removeClass('dim');
      }
    });

    // Add CSS for dim class
    cy.style()
      .selector('.dim')
      .style({ opacity: 0.25 })
      .update();
  }

  /** @param {KeyboardEvent} e */
  function handleKey(e) {
    if (e.key === 'Enter') search();
  }

  /** @param {string=} t */
  function typeBadge(t) {
    const map = {
      person: '👤', organization: '🏛️', country: '🌍', event: '⚡', location: '📍',
    };
    return map[(t ?? '').toLowerCase()] ?? '◆';
  }

  async function openArticle(e) {
    // Force re-render: null out first, then assign
    articleEntity = null;
    articleRelations = [];
    articleSummary = null;
    await tick();
    articleEntity = e;

    // Load relations + summary in parallel
    try {
      const [rels, summaryRes] = await Promise.all([
        api('/api/v1/relationships?limit=5000'),
        api(`/api/v1/entities/${encodeURIComponent(e.id)}/summary`).catch(() => null),
      ]);
      const list = Array.isArray(rels) ? rels : rels.relationships ?? [];
      const id = e.id;
      articleRelations = list
        .filter((r) => {
          const from = r.source_id ?? r.from_entity;
          const to = r.target_id ?? r.to_entity;
          return from === id || to === id;
        })
        .map((r) => {
          const from = r.source_id ?? r.from_entity;
          const to = r.target_id ?? r.to_entity;
          const relType = r.relation_type ?? r.relationship_type ?? r.type ?? '';
          return { from, to, relType, outgoing: from === id };
        });
      articleSummary = summaryRes?.summary ?? null;
    } catch {
      articleRelations = [];
      articleSummary = null;
    }
  }

  function closeArticle() {
    console.log('[drawer] closeArticle fired', articleEntity?.id, '| before null');
    try {
      articleEntity = null;
      articleRelations = [];
      articleSummary = null;
      console.log('[drawer] state after null:', articleEntity);
    } catch (err) {
      console.error('[drawer] close failed', err);
    }
  }

  // Global ESC key to close drawer
  function handleGlobalKeydown(e) {
    if (e.key === 'Escape' && articleEntity) closeArticle();
  }

  async function loadSources() {
    try {
      const s = await api('/api/v1/sources');
      allSources = Array.isArray(s) ? s : [];
      const wanted = new Set(DEFAULT_SOURCES);
      selectedSources = new Set(allSources.filter((x) => wanted.has(x)));
      if (selectedSources.size === 0) selectedSources = new Set(allSources);
    } catch {
      allSources = [];
    }
  }

  async function reloadPlugins() {
    try {
      const r = await api('/api/v1/plugins/reload', { method: 'POST' });
      await loadSources();
      if (r.failed?.length) {
        error = `Plugins en échec: ${r.failed.map((f) => f.name).join(', ')}`;
      }
    } catch (e) {
      error = `Reload plugins échoué : ${e.message}`;
    }
  }

  // Group entities by country (fallback "—" for no-country), then by source inside
  $: groupedByCountry = (() => {
    const groups = new Map(); // country -> [entities]
    for (const e of entities) {
      const country = e.country ?? e.properties?.country ?? '—';
      if (!groups.has(country)) groups.set(country, []);
      groups.get(country).push(e);
    }
    // Sort countries alphabetically, "—" last
    const sorted = [...groups.entries()].sort(([a], [b]) => {
      if (a === '—') return 1;
      if (b === '—') return -1;
      return a.localeCompare(b, 'fr');
    });
    return sorted;
  })();

  onMount(async () => {
    // Bind global ESC to close drawer
    window.addEventListener('keydown', handleGlobalKeydown);
    try {
      const h = await api('/health');
      health = typeof h === 'string' ? h : h.status ?? 'ok';
    } catch {
      health = 'hors ligne';
    }
    // Load sources, set smart defaults
    await loadSources();
    search();
    return () => window.removeEventListener('keydown', handleGlobalKeydown);
  });
</script>

<main>
  <header>
    <div class="brand">
      <h1>🐓 Le Coq</h1>
      <p class="subtitle">Plateforme de Renseignement Géopolitique</p>
    </div>
    <div class="health" class:down={health === 'hors ligne'}>
      ● {health || '…'}
    </div>
  </header>

  <div class="toolbar">
    <input
      type="text"
      bind:value={query}
      on:keydown={handleKey}
      placeholder="Rechercher entités, organisations, événements…"
    />
    <label class="online-toggle" title="Inclure les sources externes (ingestion auto en cache)">
      <input type="checkbox" bind:checked={ingestOnline} />
      🌐 Sources externes
    </label>
    <button on:click={search} disabled={loading}>
      {loading && searching ? '🌐 Recherche…' : loading ? '…' : 'Rechercher'}
    </button>
    <button class="secondary" on:click={loadGraph} disabled={loading}>
      Vue Graphe
    </button>
    <button class="secondary focus-btn" on:click={loadFocus} disabled={loading || !query.trim()} title="Cas 10-min : smart search + graphe centré sur l'entité">
      🎯 Vue Focus
    </button>
    <button class="secondary" on:click={loadMap} disabled={loading} title="Carte monde choroplèthe par densité d'entités">
      🗺️ Carte
    </button>
    <button class="secondary" on:click={() => (view = 'list')} disabled={view === 'list'}>
      Vue Liste
    </button>
  </div>

  {#if ingestOnline && allSources.length}
    <div class="source-chips">
      <span class="chips-label">Sources :</span>
      {#each allSources as s}
        <button
          type="button"
          class="chip"
          class:active={selectedSources.has(s)}
          on:click={() => {
            if (selectedSources.has(s)) selectedSources.delete(s);
            else selectedSources.add(s);
            selectedSources = selectedSources; // trigger reactivity
          }}
        >
          {s}
        </button>
      {/each}
      <button type="button" class="chip reload" on:click={reloadPlugins} title="Recharger les plugins (hot-reload)">
        ♻️ reload
      </button>
    </div>
  {/if}

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if searchErrors.length}
    <div class="warn">⚠️ {searchErrors.join(' · ')}</div>
  {/if}

  {#if newItems > 0 && !error}
    <div class="info">🆕 {newItems} élément{newItems !== 1 ? 's' : ''} ingéré{newItems !== 1 ? 's' : ''} depuis les sources externes</div>
  {/if}

  {#if view === 'list'}
    <section class="results">
      <p class="count">{entities.length} entité{entities.length !== 1 ? 's' : ''} · {groupedByCountry.length} pays</p>
      {#each groupedByCountry as [country, list] (country)}
        <div class="country-group">
          <h2 class="country-title">
            🌍 {country} <span class="country-count">({list.length})</span>
          </h2>
          <div class="country-grid">
            {#each list as e (e.id)}
              {@const desc = e.description ?? e.properties?.description}
              {@const src = e.properties?.source}
              <article class="card clickable" on:click={() => openArticle(e)} role="button" tabindex="0" on:keydown={(ev) => ev.key === 'Enter' && openArticle(e)}>
                <div class="card-head">
                  <span class="badge">{typeBadge(e.entity_type ?? e.type)} {e.entity_type ?? e.type ?? 'entité'}</span>
                  <span class="card-meta">
                    {#if src}<span class="src-badge">{src}</span>{/if}
                  </span>
                </div>
                <h3>{e.name}</h3>
                {#if desc}<p>{desc}</p>{/if}
                <span class="card-action">→ Lire</span>
              </article>
            {/each}
          </div>
        </div>
      {:else}
        {#if !loading}<p class="empty">Aucune entité. Lancez une ingestion via le CLI.</p>{/if}
      {/each}
    </section>
  {:else if view === 'map'}
    <div class="map-toolbar">
      <span class="graph-stats">🗺️ Carte — densité d'entités par pays · cliquer pour voir les détails</span>
    </div>
    <div class="map-wrap" bind:this={mapEl}></div>

    {#if mapInfo}
      <aside class="country-panel">
        <div class="country-head">
          <h3>🌍 {mapInfo.name}</h3>
          <button class="close" on:click={() => (mapInfo = null)} aria-label="Fermer">×</button>
        </div>
        <p class="country-meta">
          {mapInfo.entities.length > 0
            ? `${mapInfo.entities.length} entité${mapInfo.entities.length > 1 ? 's' : ''} dans la base`
            : 'Aucune entité dans la base'}
        </p>
        {#if mapInfo.entities.length > 0}
          <ul class="country-list">
            {#each mapInfo.entities.slice(0, 30) as e (e.id)}
              <li class="country-item">
                <span class="badge">{e.type ?? e.entity_type ?? '—'}</span>
                <span class="country-item-name">{e.name}</span>
              </li>
            {/each}
            {#if mapInfo.entities.length > 30}
              <li class="country-item dim">… et {mapInfo.entities.length - 30} autres</li>
            {/if}
          </ul>
          <div class="drawer-actions">
            <button class="secondary small" on:click={() => { query = mapInfo.name; mapInfo = null; view = 'list'; search(); }}>
              🔎 Rechercher "{mapInfo.name}"
            </button>
          </div>
        {:else}
          <div class="drawer-actions">
            <button class="secondary small" on:click={async () => { query = mapInfo.name; ingestOnline = true; mapInfo = null; view = 'list'; await search(); }}>
              🌐 Lancer une recherche en ligne sur "{mapInfo.name}"
            </button>
          </div>
        {/if}
      </aside>
    {/if}
  {:else}
    <div class="graph-toolbar">
      <span class="graph-stats">{nodeCount} nœuds · {edgeCount} arêtes</span>
      <button class="secondary small" on:click={() => { showIsolated = !showIsolated; renderGraph(); }} title="Afficher/masquer les entités sans lien">
        {showIsolated ? '👁️ Masquer orphelins' : '👁️‍🗨️ Tout afficher'}
      </button>
      <button class="secondary small" on:click={() => { hideWeakLinks = !hideWeakLinks; renderGraph(); }} title="Masquer les liens 'related' vers stubs (bruit Wikipedia)">
        {hideWeakLinks ? '🔗 Liens forts' : '🔗 Tous liens'}
      </button>
      <span class="year-slider" title={`Afficher uniquement les données jusqu'à ${maxYear}`}>
        <span class="year-label">📅 {maxYear}</span>
        <input
          type="range"
          min={MIN_YEAR}
          max={new Date().getFullYear()}
          bind:value={maxYear}
          on:input={renderGraph}
        />
      </span>
      <select bind:value={graphLayout} on:change={renderGraph}>
        <option value="cose">cose</option>
        <option value="breadthfirst">hiérarchie</option>
        <option value="concentric">concentrique</option>
        <option value="circle">cercle</option>
      </select>
      <button class="secondary small" on:click={() => cy?.fit(undefined, 30)}>⟲ Recentrer</button>
    </div>
    <div class="graph-container">
      <div class="graph-wrap" bind:this={graphEl}></div>
      {#if selectedNode}
        <aside class="node-panel">
          <div class="panel-head">
            <h3>{selectedNode.label ?? selectedNode.id ?? 'Entité'}</h3>
            <button class="close" on:click={() => (selectedNode = null)}>×</button>
          </div>
          <div class="panel-body">
            <div class="kv"><span class="k">Type</span><span class="v badge">{selectedNode.type}</span></div>
            {#if selectedNode.country}<div class="kv"><span class="k">Pays</span><span class="v">{selectedNode.country}</span></div>{/if}
            {#if selectedNode.source}<div class="kv"><span class="k">Source</span><span class="v src-badge">{selectedNode.source}</span></div>{/if}
            <div class="kv"><span class="k">Connexions</span><span class="v">{selectedNode.degree}</span></div>
            {#if selectedNode.description}
              <p class="desc">{selectedNode.description.slice(0, 280)}{selectedNode.description.length > 280 ? '…' : ''}</p>
            {/if}
            <div class="panel-actions">
              <button class="secondary small" on:click={() => { query = selectedNode.label; ingestOnline = true; view = 'list'; search(); }}>
                🔎 Explorer depuis ce nœud
              </button>
              {#if selectedNode.placeholder === 'true' || selectedNode.type === 'stub'}
                <button
                  class="secondary small enrich"
                  on:click={async () => {
                    query = selectedNode.label;
                    ingestOnline = true;
                    await search();
                    // Re-render graph to pick up upgraded entity
                    if (view === 'graph' || view === 'focus') { await loadGraph(); }
                  }}
                  title="Lance une recherche en ligne pour remplacer ce stub par une vraie entité"
                >
                  ⬆️ Enrichir ce stub
                </button>
              {/if}
            </div>
          </div>
        </aside>
      {/if}
    </div>
  {/if}
</main>

<!-- Article drawer: side panel with entity details + relations -->
{#if articleEntity}
  {@const drawerKey = articleEntity.id}
  <div class="drawer-overlay" on:click={closeArticle} on:keydown={(e) => e.key === 'Escape' && closeArticle()} role="presentation" tabindex="-1"></div>
  <aside class="article-drawer" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Détail de l'entité" data-id={drawerKey}>
    <button class="close-drawer" type="button" on:click={(e) => { e.stopPropagation(); console.log('[close] clicked'); closeArticle(); }} aria-label="Fermer">×</button>
    <div class="drawer-head">
      <span class="badge">{typeBadge(articleEntity.entity_type ?? articleEntity.type)} {articleEntity.entity_type ?? articleEntity.type ?? 'entité'}</span>
    </div>
    <h2 class="drawer-title">{articleEntity.name}</h2>

    {#if articleSummary}
      <section class="drawer-summary">
        <h3 class="section-title">📖 Résumé</h3>
        <p class="summary-text">{articleSummary}</p>
      </section>
    {/if}

    {#if articleEntity.description ?? articleEntity.properties?.description}
      <section class="drawer-section">
        <h3 class="section-title">Description</h3>
        <p class="drawer-desc">{articleEntity.description ?? articleEntity.properties?.description}</p>
      </section>
    {/if}

    {#if articleEntity.country ?? articleEntity.properties?.country}
      <div class="drawer-kv"><span class="k">Pays</span><span class="v">{articleEntity.country ?? articleEntity.properties?.country}</span></div>
    {/if}
    {#if articleEntity.properties?.source}
      <div class="drawer-kv"><span class="k">Source</span><span class="v src-badge">{articleEntity.properties.source}</span></div>
    {/if}

    {#if articleEntity.properties && Object.keys(articleEntity.properties).filter((k) => !['description', 'country', 'source'].includes(k)).length > 0}
      <section class="drawer-section">
        <h3 class="section-title">Propriétés</h3>
        {#each Object.entries(articleEntity.properties).filter(([k]) => !['description', 'country', 'source'].includes(k)) as [k, v]}
          <div class="drawer-kv">
            <span class="k">{k}</span>
            <span class="v">{typeof v === 'object' ? JSON.stringify(v) : String(v)}</span>
          </div>
        {/each}
      </section>
    {/if}

    {#if articleRelations.length > 0}
      <section class="drawer-section">
        <h3 class="section-title">Relations ({articleRelations.length})</h3>
        <ul class="rel-list">
          {#each articleRelations as r (r.from + '|' + r.relType + '|' + r.to)}
            <li class="rel-item">
              {#if r.outgoing}
                <span class="rel-dir">→</span> <strong>{r.relType}</strong> <span class="rel-target">{r.to.replace(/^wiki_|^wd_item_|^uncomtrade_country_|^eurostat_country_/, '').replace(/_/g, ' ')}</span>
              {:else}
                <span class="rel-dir">←</span> <strong>{r.relType}</strong> <span class="rel-target">{r.from.replace(/^wiki_|^wd_item_|^uncomtrade_country_|^eurostat_country_/, '').replace(/_/g, ' ')}</span>
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <div class="drawer-actions">
      <button class="secondary small" on:click={async () => { query = articleEntity.name; const id = articleEntity.id; closeArticle(); view = 'graph'; await loadFocus(true, id); }}>
        🎯 Ouvrir en Vue Focus
      </button>
    </div>
  </aside>
{/if}

<style>
  /* Design tokens */
  :global(:root) {
    --bg: #0a0a0f;
    --surface: rgba(255, 255, 255, 0.07);
    --surface-hover: rgba(255, 255, 255, 0.12);
    --border: rgba(255, 255, 255, 0.14);
    --border-strong: rgba(255, 255, 255, 0.24);
    --text: rgba(255, 255, 255, 0.95);
    --text-2: rgba(255, 255, 255, 0.6);
    --text-3: rgba(255, 255, 255, 0.4);
    --accent: #2dd4bf;          /* teal — géopolitique sérieux, confiance */
    --accent-soft: rgba(45, 212, 191, 0.15);
    --accent-strong: rgba(45, 212, 191, 0.35);
    --gold: #c9a227;            /* gardé en secondaire pour la marque */
    --danger: #f87171;
    --radius-sm: 10px;
    --radius: 16px;
    --radius-lg: 20px;
    --ease: cubic-bezier(0.4, 0, 0.2, 1);
  }

  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--text);
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, system-ui, sans-serif;
    font-size: 15px;
    -webkit-font-smoothing: antialiased;
  }

  /* Liquid glass ambient glow — teal light from the top */
  :global(body)::before {
    content: '';
    position: fixed;
    top: -280px;
    left: 50%;
    transform: translateX(-50%);
    width: min(1300px, 130vw);
    height: 700px;
    background: radial-gradient(
      ellipse at center,
      rgba(45, 212, 191, 0.28) 0%,
      rgba(45, 212, 191, 0.14) 35%,
      transparent 70%
    );
    filter: blur(80px);
    pointer-events: none;
    z-index: 0;
  }

  /* Secondary softer glow bottom-right for depth */
  :global(body)::after {
    content: '';
    position: fixed;
    bottom: -250px;
    right: -150px;
    width: 800px;
    height: 550px;
    background: radial-gradient(
      ellipse at center,
      rgba(45, 212, 191, 0.11) 0%,
      transparent 60%
    );
    filter: blur(100px);
    pointer-events: none;
    z-index: 0;
  }

  :global(#app) {
    position: relative;
    z-index: 1;
  }

  main { max-width: 1400px; margin: 0 auto; padding: 2rem 1.5rem; }
  header { display: flex; justify-content: space-between; align-items: start; margin-bottom: 2.5rem; }

  h1 {
    margin: 0; font-size: 1.75rem; font-weight: 700;
    letter-spacing: -0.03em; color: var(--text);
  }
  .subtitle { margin: 0.3rem 0 0; color: var(--text-2); font-size: 0.92rem; font-weight: 400; }
  .health {
    font-size: 0.8rem; color: var(--accent); padding: 0.4rem 0.9rem;
    background: var(--accent-soft); border-radius: 999px; border: 1px solid var(--accent-strong);
  }
  .health.down { color: var(--danger); background: rgba(248,113,113,0.1); border-color: rgba(248,113,113,0.3); }

  /* Toolbar: glass pill bar */
  .toolbar {
    display: flex; gap: 0.5rem; margin-bottom: 1rem;
    padding: 0.5rem; background: var(--surface);
    border: 1px solid var(--border); border-radius: var(--radius-lg);
    backdrop-filter: blur(20px); -webkit-backdrop-filter: blur(20px);
  }
  input {
    flex: 1; padding: 0.65rem 1rem; border-radius: 999px;
    border: 1px solid transparent; background: transparent;
    color: var(--text); font-size: 0.95rem; outline: none;
    transition: border-color 200ms var(--ease);
  }
  input::placeholder { color: var(--text-3); }
  input:focus { border-color: var(--accent-strong); }

  button {
    padding: 0.65rem 1.2rem; border-radius: 999px; border: none; cursor: pointer;
    background: var(--accent); color: var(--bg);
    font-weight: 600; font-size: 0.9rem;
    transition: all 200ms var(--ease);
  }
  button:hover:not(:disabled) { transform: translateY(-1px); box-shadow: 0 4px 16px var(--accent-soft); }
  button:disabled { opacity: 0.4; cursor: default; }
  button.secondary {
    background: var(--surface); color: var(--text);
    border: 1px solid var(--border);
  }
  button.secondary:hover:not(:disabled) { background: var(--surface-hover); box-shadow: none; }
  button.small { padding: 0.4rem 0.85rem; font-size: 0.82rem; }

  .online-toggle {
    display: flex; align-items: center; gap: 0.4rem; padding: 0 0.85rem;
    background: transparent; border-radius: 999px;
    font-size: 0.88rem; cursor: pointer; white-space: nowrap; color: var(--text-2);
  }
  .online-toggle input { accent-color: var(--accent); margin: 0; }
  .online-toggle input[type="checkbox"] { width: auto; flex: none; }

  select {
    padding: 0.5rem 0.85rem; border-radius: 999px;
    border: 1px solid var(--border); background: var(--surface);
    color: var(--text); font-size: 0.85rem; outline: none; cursor: pointer;
  }
  select:focus { border-color: var(--accent-strong); }

  /* Alerts */
  .error, .info, .warn {
    padding: 0.75rem 1.1rem; border-radius: var(--radius);
    margin-bottom: 1rem; font-size: 0.9rem;
    backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
  }
  .error { background: rgba(248,113,113,0.08); border: 1px solid rgba(248,113,113,0.25); color: #fca5a5; }
  .info  { background: rgba(74,222,128,0.08); border: 1px solid rgba(74,222,128,0.25); color: #86efac; }
  .warn  { background: rgba(251,191,36,0.08); border: 1px solid rgba(251,191,36,0.25); color: #fcd34d; }

  /* Source chips */
  .source-chips {
    display: flex; align-items: center; flex-wrap: wrap; gap: 0.4rem;
    margin-bottom: 1rem; padding: 0.25rem 0;
  }
  .chips-label { color: var(--text-3); font-size: 0.82rem; margin-right: 0.3rem; }
  .chip {
    padding: 0.35rem 0.85rem; border-radius: 999px;
    border: 1px solid var(--border); background: var(--surface);
    color: var(--text-2); font-size: 0.8rem; cursor: pointer;
    transition: all 200ms var(--ease); font-weight: 500;
  }
  .chip:hover { border-color: var(--border-strong); color: var(--text); }
  .chip.active {
    background: var(--accent-soft); border-color: var(--accent-strong); color: var(--accent);
  }

  .count { color: var(--text-3); font-size: 0.85rem; margin: 0 0 1rem; }

  /* Country groups */
  .results { display: flex; flex-direction: column; gap: 2rem; }
  .country-group { display: flex; flex-direction: column; gap: 0.85rem; }
  .country-title {
    margin: 0; font-size: 1.05rem; font-weight: 600; color: var(--text);
    letter-spacing: -0.02em;
  }
  .country-count { color: var(--text-3); font-size: 0.88rem; font-weight: 400; margin-left: 0.3rem; }
  .country-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 0.85rem; }

  /* Cards: glass */
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 1.1rem 1.3rem;
    backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px);
    box-shadow: 0 4px 16px rgba(0,0,0,0.25);
  }
  .card.clickable {
    cursor: pointer;
    transition: all 200ms var(--ease);
  }
  .card.clickable:hover {
    background: var(--surface-hover);
    border-color: var(--border-strong);
    transform: translateY(-2px);
    box-shadow: 0 12px 32px rgba(0,0,0,0.4), 0 0 24px rgba(45,212,191,0.1);
  }
  .card-action {
    display: inline-block; margin-top: 0.6rem;
    font-size: 0.8rem; color: var(--accent);
    opacity: 0; transition: opacity 200ms var(--ease);
  }
  .card.clickable:hover .card-action { opacity: 1; }
  .card-head { display: flex; justify-content: space-between; margin-bottom: 0.55rem; align-items: center; }
  .card h3 {
    margin: 0 0 0.4rem; font-size: 1rem; font-weight: 600;
    letter-spacing: -0.015em; color: var(--text);
  }
  .card p { margin: 0; color: var(--text-2); font-size: 0.88rem; line-height: 1.5; }
  .card-meta { display: flex; gap: 0.4rem; align-items: center; }

  /* Badges */
  .badge {
    font-size: 0.7rem; font-weight: 600;
    color: var(--accent); background: var(--accent-soft);
    padding: 0.2rem 0.65rem; border-radius: 999px;
  }
  .src-badge {
    font-size: 0.7rem; color: var(--text-3);
    background: var(--surface); border: 1px solid var(--border);
    padding: 0.2rem 0.6rem; border-radius: 999px;
  }

  .empty { color: var(--text-3); font-style: italic; }

  /* Focus button */
  .focus-btn { background: var(--accent-soft); color: var(--accent); border: 1px solid var(--accent-strong); }
  .focus-btn:hover:not(:disabled) { background: var(--accent); color: var(--bg); }
  .focus-btn:disabled { opacity: 0.3; }

  /* Graph */
  .graph-wrap {
    height: 70vh;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    flex: 1; min-width: 0;
    backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px);
    box-shadow: inset 0 1px 0 rgba(255,255,255,0.1), 0 8px 32px rgba(0,0,0,0.3);
  }
  .graph-toolbar {
    display: flex; align-items: center; gap: 0.6rem; margin-bottom: 0.75rem;
    padding: 0.5rem 0.85rem;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 999px;
    backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px);
    box-shadow: 0 4px 16px rgba(0,0,0,0.2);
  }
  .map-toolbar {
    display: flex; align-items: center; gap: 0.6rem; margin-bottom: 0.75rem;
    padding: 0.5rem 0.85rem;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 999px;
    backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px);
    box-shadow: 0 4px 16px rgba(0,0,0,0.2);
  }
  .map-wrap {
    height: 75vh;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px);
    box-shadow: inset 0 1px 0 rgba(255,255,255,0.1), 0 8px 32px rgba(0,0,0,0.3);
    overflow: hidden;
  }
  .country-panel {
    position: fixed;
    bottom: 1.5rem;
    left: 1.5rem;
    width: 360px;
    max-height: 60vh;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    padding: 1.1rem 1.3rem;
    backdrop-filter: blur(20px); -webkit-backdrop-filter: blur(20px);
    box-shadow: 0 12px 32px rgba(0,0,0,0.5), 0 0 24px rgba(45,212,191,0.1);
    z-index: 50;
    animation: slideUp 250ms var(--ease);
  }
  @keyframes slideUp {
    from { transform: translateY(20px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
  .country-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  .country-head h3 {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.015em;
  }
  .country-meta {
    margin: 0 0 0.75rem;
    color: var(--text-2);
    font-size: 0.85rem;
  }
  .country-list { margin: 0; padding: 0; list-style: none; }
  .country-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border);
  }
  .country-item:last-child { border-bottom: none; }
  .country-item-name {
    color: var(--text);
    font-size: 0.92rem;
    word-break: break-word;
    flex: 1;
  }
  .country-item.dim { color: var(--text-3); font-style: italic; }
  .graph-stats { color: var(--text-3); font-size: 0.82rem; margin-right: auto; }
  .year-slider {
    display: flex; align-items: center; gap: 0.5rem;
    padding: 0.25rem 0.6rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
  }
  .year-label { font-size: 0.8rem; color: var(--text-2); white-space: nowrap; }
  .year-slider input[type='range'] {
    -webkit-appearance: none; appearance: none;
    width: 130px; height: 4px;
    background: var(--border-strong); border-radius: 2px;
    outline: none; cursor: pointer;
    accent-color: var(--accent);
    padding: 0; border-radius: 2px;
  }
  .year-slider input[type='range']::-webkit-slider-thumb {
    -webkit-appearance: none; appearance: none;
    width: 14px; height: 14px;
    border-radius: 50%; background: var(--accent);
    border: 2px solid var(--bg); cursor: pointer;
  }
  .year-slider input[type='range']::-moz-range-thumb {
    width: 14px; height: 14px;
    border-radius: 50%; background: var(--accent);
    border: 2px solid var(--bg); cursor: pointer;
  }
  .graph-container {
    display: flex; gap: 1rem; align-items: stretch;
    width: 100%; max-width: 100%; overflow: hidden;
  }

  /* Node panel */
  .node-panel {
    width: 320px; min-width: 280px; max-width: 320px; flex-shrink: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 1.1rem 1.25rem; overflow-y: auto; max-height: 70vh;
    word-break: break-word; overflow-wrap: break-word;
    backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px);
    box-shadow: 0 8px 24px rgba(0,0,0,0.3);
  }
  .node-panel .desc { word-break: break-word; }
  .panel-head { display: flex; justify-content: space-between; align-items: start; margin-bottom: 0.75rem; gap: 0.5rem; }
  .panel-head h3 {
    margin: 0; font-size: 1.1rem; font-weight: 600; color: var(--text);
    letter-spacing: -0.015em;
    word-break: break-word; overflow-wrap: break-word;
    flex: 1; min-width: 0;
  }
  .close {
    background: var(--surface); color: var(--text-3); border: 1px solid var(--border);
    font-size: 1.4rem; border-radius: 999px;
    cursor: pointer; padding: 0.25rem 0.5rem; line-height: 1;
    transition: all 200ms var(--ease);
    flex-shrink: 0;
  }
  .close:hover { color: var(--text); background: var(--surface-hover); border-color: var(--border-strong); }
  .kv {
    display: flex; justify-content: space-between; align-items: center;
    padding: 0.45rem 0; border-bottom: 1px solid var(--border); font-size: 0.85rem;
  }
  .kv .k { color: var(--text-3); font-size: 0.78rem; }
  .kv .v { color: var(--text); }
  .desc { color: var(--text-2); font-size: 0.88rem; line-height: 1.55; margin-top: 0.75rem; }
  .panel-actions { margin-top: 1rem; display: flex; flex-direction: column; gap: 0.5rem; }

  /* Article drawer */
  .drawer-overlay {
    position: fixed; inset: 0;
    background: rgba(5, 5, 10, 0.65);
    z-index: 90;
    backdrop-filter: blur(6px); -webkit-backdrop-filter: blur(6px);
    animation: fadeIn 200ms var(--ease);
  }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes slideIn { from { transform: translateX(40px); opacity: 0; } to { transform: translateX(0); opacity: 1; } }

  .article-drawer {
    position: fixed; top: 0; right: 0; bottom: 0;
    width: 480px; max-width: 90vw;
    background: rgba(18, 20, 28, 0.92);
    border-left: 1px solid var(--border-strong);
    padding: 3.5rem 1.75rem 1.6rem;
    overflow-y: auto; z-index: 100;
    backdrop-filter: blur(24px); -webkit-backdrop-filter: blur(24px);
    box-shadow: -16px 0 48px rgba(0, 0, 0, 0.5), inset 1px 0 0 rgba(255,255,255,0.05);
    animation: slideIn 200ms var(--ease);
  }
  /* Floating close button — always top-right, always clickable */
  .close-drawer {
    position: fixed; top: 1.25rem; right: 1.25rem;
    width: 44px; height: 44px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--text);
    font-size: 1.6rem; line-height: 1;
    cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px);
    box-shadow: 0 4px 16px rgba(0,0,0,0.4);
    transition: all 200ms var(--ease);
    z-index: 200;
  }
  .close-drawer:hover {
    background: var(--accent);
    color: var(--bg);
    transform: scale(1.05);
  }
  .drawer-head {
    display: flex; justify-content: space-between; align-items: start;
    margin-bottom: 0.75rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border);
  }
  .drawer-title {
    margin: 0 0 1.1rem; font-size: 1.45rem; font-weight: 700;
    color: var(--text); letter-spacing: -0.02em;
    line-height: 1.2; word-break: break-word;
  }
  .drawer-section { margin-top: 1.4rem; }
  .section-title {
    margin: 0 0 0.55rem; font-size: 0.75rem; font-weight: 600;
    text-transform: uppercase; letter-spacing: 0.08em;
    color: var(--text-3);
  }
  .drawer-desc {
    margin: 0; color: var(--text); line-height: 1.65; font-size: 0.93rem;
    word-break: break-word;
  }
  .drawer-kv {
    display: flex; justify-content: space-between; align-items: start;
    padding: 0.45rem 0; border-bottom: 1px solid var(--border);
    gap: 0.75rem; font-size: 0.87rem;
  }
  .drawer-kv .k { color: var(--text-3); text-transform: capitalize; flex-shrink: 0; min-width: 100px; }
  .drawer-kv .v { color: var(--text); text-align: right; word-break: break-word; }
  .rel-list { margin: 0; padding: 0; list-style: none; }
  .rel-item {
    padding: 0.5rem 0; border-bottom: 1px solid var(--border);
    font-size: 0.87rem; display: flex; gap: 0.45rem; align-items: baseline;
  }
  .rel-dir { color: var(--accent); font-weight: 700; }
  .rel-target { color: var(--text-2); margin-left: auto; text-align: right; word-break: break-word; }
  .drawer-actions {
    margin-top: 1.5rem; padding-top: 1rem;
    border-top: 1px solid var(--border);
    display: flex; flex-direction: column; gap: 0.5rem;
  }

  /* Narrative summary block */
  .drawer-summary {
    margin-top: 0.5rem;
    padding: 0.9rem 1rem;
    background: var(--accent-soft);
    border: 1px solid var(--accent-strong);
    border-radius: var(--radius);
    border-left: 3px solid var(--accent);
  }
  .summary-text {
    margin: 0;
    color: var(--text);
    font-size: 0.92rem;
    line-height: 1.65;
    font-style: italic;
  }
</style>
