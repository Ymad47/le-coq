<script>
  import { onMount, tick } from 'svelte';
  import cytoscape from 'cytoscape';

  let entities = [];
  let relationships = [];
  let query = '';
  let loading = false;
  let error = '';
  let view = 'list'; // 'list' | 'graph' | 'focus'
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
  async function loadFocus() {
    if (!query.trim()) {
      error = 'Entrez une recherche d\'abord pour utiliser Vue Focus.';
      return;
    }
    loading = true;
    searching = true;
    error = '';
    newItems = 0;
    searchErrors = [];

    try {
      // 1) Smart search first (ingests + refreshes)
      const params = new URLSearchParams({ q: query.trim(), online: ingestOnline });
      if (ingestOnline && selectedSources.size > 0) {
        params.set('sources', Array.from(selectedSources).join(','));
      }
      const data = await api(`/api/v1/search?${params}`);
      const results = data.results ?? [];
      newItems = data.new_items ?? 0;
      searchErrors = data.errors ?? [];

      // 2) Load full graph for visualization
      const [ents, rels] = await Promise.all([
        api('/api/v1/entities?limit=1000'),
        api('/api/v1/relationships?limit=5000'),
      ]);
      entities = Array.isArray(ents) ? ents : ents.entities ?? [];
      relationships = Array.isArray(rels) ? rels : rels.relationships ?? [];

      // 3) Identify hub: prefer 1st match by name; fallback to highest degree
      const qLower = query.trim().toLowerCase();
      let hub = null;
      // exact / starts-with
      hub = results.find((r) => (r.name ?? '').toLowerCase() === qLower)
        ?? results.find((r) => (r.name ?? '').toLowerCase().startsWith(qLower))
        ?? results[0];
      focusHubId = hub?.id ?? null;

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
    parent_of: '#c9a227',
    exports_to: '#4299e1',
    imports_from: '#4299e1',
    has_indicator: '#4a5568',
    default: '#4a5568',
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
  let graphLayout = 'cose';
  let nodeCount = 0;
  let edgeCount = 0;
  let showIsolated = false; // hide degree-0 nodes by default
  let hideWeakLinks = true; // hide 'related_to' edges pointing to stubs

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

    // Edges: accept both old and new keys, filter weak links if enabled
    const edges = relationships
      .map((r) => {
        const from = r.source_id ?? r.from_entity;
        const to = r.target_id ?? r.to_entity;
        const relType = r.relation_type ?? r.relationship_type ?? r.type ?? '';
        return { from, to, relType, rawId: r.id };
      })
      .filter((e) => e.from && e.to)
      .map((e) => ({
        data: {
          id: e.rawId,
          source: e.from,
          target: e.to,
          label: e.relType,
          relType: e.relType,
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
            color: '#e8e6e3',
            'font-size': 10,
            'font-weight': 500,
            'text-valign': 'bottom',
            'text-margin-y': 4,
            'text-wrap': 'ellipsis',
            'text-max-width': '80px',
            width: 'data(size)',
            height: 'data(size)',
            'border-width': 0,
            'border-color': '#c9a227',
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
            'border-color': '#f6ad37',
            'border-opacity': 0.9,
          },
        },
        {
          selector: 'node:selected',
          style: {
            'border-width': 3,
            'border-color': '#c9a227',
            'overlay-color': '#c9a227',
            'overlay-opacity': 0.15,
          },
        },
        {
          selector: 'node:active',
          style: {
            'overlay-color': '#c9a227',
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
            color: '#a0aec0',
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

  onMount(async () => {
    try {
      const h = await api('/health');
      health = typeof h === 'string' ? h : h.status ?? 'ok';
    } catch {
      health = 'hors ligne';
    }
    // Load sources, set smart defaults
    await loadSources();
    search();
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
      <p class="count">{entities.length} entité{entities.length !== 1 ? 's' : ''}</p>
      {#each entities as e (e.id)}
        {@const desc = e.description ?? e.properties?.description}
        {@const src = e.properties?.source}
        <article class="card">
          <div class="card-head">
            <span class="badge">{typeBadge(e.entity_type ?? e.type)} {e.entity_type ?? e.type ?? 'entité'}</span>
            <span class="card-meta">
              {#if src}<span class="src-badge">{src}</span>{/if}
              {#if e.country ?? e.properties?.country}<span class="country">{e.country ?? e.properties?.country}</span>{/if}
            </span>
          </div>
          <h3>{e.name}</h3>
          {#if desc}<p>{desc}</p>{/if}
        </article>
      {:else}
        {#if !loading}<p class="empty">Aucune entité. Lancez une ingestion via le CLI.</p>{/if}
      {/each}
    </section>
  {:else}
    <div class="graph-toolbar">
      <span class="graph-stats">{nodeCount} nœuds · {edgeCount} arêtes</span>
      <button class="secondary small" on:click={() => { showIsolated = !showIsolated; renderGraph(); }} title="Afficher/masquer les entités sans lien">
        {showIsolated ? '👁️ Masquer orphelins' : '👁️‍🗨️ Tout afficher'}
      </button>
      <button class="secondary small" on:click={() => { hideWeakLinks = !hideWeakLinks; renderGraph(); }} title="Masquer les liens 'related' vers stubs (bruit Wikipedia)">
        {hideWeakLinks ? '🔗 Liens forts' : '🔗 Tous liens'}
      </button>
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

<style>
  :global(body) {
    margin: 0;
    background: #0f1115;
    color: #e8e6e3;
    font-family: 'Inter', system-ui, sans-serif;
  }
  main { max-width: 1100px; margin: 0 auto; padding: 2rem 1.5rem; }
  header { display: flex; justify-content: space-between; align-items: start; margin-bottom: 2rem; }
  h1 { margin: 0; font-size: 2rem; letter-spacing: -0.02em; }
  .subtitle { margin: 0.25rem 0 0; color: #a0aec0; font-size: 0.95rem; }
  .health { font-size: 0.85rem; color: #68d391; padding-top: 0.5rem; }
  .health.down { color: #fc8181; }
  .toolbar { display: flex; gap: 0.5rem; margin-bottom: 1.25rem; }
  input {
    flex: 1; padding: 0.7rem 1rem; border-radius: 8px; border: 1px solid #2d3748;
    background: #1a202c; color: #e8e6e3; font-size: 1rem; outline: none;
  }
  input:focus { border-color: #c9a227; }
  button {
    padding: 0.7rem 1.25rem; border-radius: 8px; border: none; cursor: pointer;
    background: #c9a227; color: #0f1115; font-weight: 600; font-size: 0.95rem;
  }
  button:disabled { opacity: 0.5; cursor: default; }
  button.secondary { background: #2d3748; color: #e8e6e3; }
  .error {
    background: #3d1f1f; border: 1px solid #c53030; color: #feb2b2;
    padding: 0.75rem 1rem; border-radius: 8px; margin-bottom: 1rem;
  }
  .count { color: #a0aec0; font-size: 0.85rem; margin: 0 0 0.75rem; }
  .results { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 1rem; }
  .card {
    background: #1a202c; border: 1px solid #2d3748; border-radius: 10px; padding: 1rem 1.25rem;
  }
  .card:hover { border-color: #c9a22755; }
  .card-head { display: flex; justify-content: space-between; margin-bottom: 0.5rem; }
  .badge {
    font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em;
    color: #c9a227; background: #c9a22718; padding: 0.15rem 0.5rem; border-radius: 4px;
  }
  .country { font-size: 0.8rem; color: #a0aec0; }
  .card h3 { margin: 0 0 0.4rem; font-size: 1.05rem; }
  .card p { margin: 0; color: #a0aec0; font-size: 0.9rem; line-height: 1.45; }
  .empty { color: #718096; font-style: italic; }
  .online-toggle {
    display: flex; align-items: center; gap: 0.4rem; padding: 0 0.75rem;
    background: #1a202c; border: 1px solid #2d3748; border-radius: 8px;
    font-size: 0.9rem; cursor: pointer; white-space: nowrap;
  }
  .online-toggle input { accent-color: #c9a227; margin: 0; }
  .online-toggle input[type="checkbox"] { width: auto; flex: none; }
  select {
    padding: 0.7rem 0.9rem; border-radius: 8px; border: 1px solid #2d3748;
    background: #1a202c; color: #e8e6e3; font-size: 0.95rem; outline: none; cursor: pointer;
  }
  select:focus { border-color: #c9a227; }
  .info {
    background: #1c2a1f; border: 1px solid #2f855a; color: #9ae6b4;
    padding: 0.75rem 1rem; border-radius: 8px; margin-bottom: 1rem;
  }
  .warn {
    background: #2d2a1c; border: 1px solid #b7791f; color: #fbd38d;
    padding: 0.75rem 1rem; border-radius: 8px; margin-bottom: 1rem;
  }
  .source-chips {
    display: flex; align-items: center; flex-wrap: wrap; gap: 0.4rem;
    margin-bottom: 1rem; padding: 0.4rem 0;
  }
  .chips-label { color: #a0aec0; font-size: 0.85rem; margin-right: 0.3rem; }
  .chip {
    padding: 0.35rem 0.75rem; border-radius: 999px; border: 1px solid #2d3748;
    background: #1a202c; color: #a0aec0; font-size: 0.8rem; cursor: pointer;
    transition: all 0.15s ease; font-weight: 500;
  }
  .chip:hover { border-color: #c9a22766; color: #e8e6e3; }
  .chip.active {
    background: #c9a22722; border-color: #c9a227; color: #c9a227;
  }
  .card-meta { display: flex; gap: 0.5rem; align-items: center; }
  .src-badge {
    font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em;
    color: #a0aec0; background: #2d374855; padding: 0.15rem 0.5rem; border-radius: 4px;
    border: 1px solid #2d3748;
  }
  .focus-btn {
    background: #c9a22755; color: #c9a227;
    border: 1px solid #c9a22744;
  }
  .focus-btn:hover:not(:disabled) { background: #c9a227; color: #0f1115; }
  .focus-btn:disabled { opacity: 0.4; }
  .graph-wrap {
    height: 70vh; background: #141822; border: 1px solid #2d3748; border-radius: 10px;
    flex: 1;
  }
  .graph-toolbar {
    display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem;
    padding: 0.5rem 0.75rem; background: #1a202c; border: 1px solid #2d3748;
    border-radius: 8px;
  }
  .graph-stats { color: #a0aec0; font-size: 0.85rem; margin-right: auto; }
  .graph-toolbar select { padding: 0.4rem 0.6rem; font-size: 0.85rem; }
  button.small { padding: 0.4rem 0.75rem; font-size: 0.85rem; }
  .graph-container { display: flex; gap: 1rem; align-items: stretch; }
  .node-panel {
    width: 320px;
    min-width: 280px;
    flex-shrink: 0;
    background: #1a202c; border: 1px solid #c9a22733;
    border-radius: 10px; padding: 1rem 1.25rem; overflow-y: auto; max-height: 70vh;
    /* keep panel readable even on narrow viewports */
    word-break: break-word;
  }
  .node-panel .desc { word-break: break-word; }
  .panel-head { display: flex; justify-content: space-between; align-items: start; margin-bottom: 0.75rem; }
  .panel-head h3 { margin: 0; font-size: 1.1rem; color: #c9a227; }
  .close {
    background: transparent; color: #a0aec0; border: none; font-size: 1.5rem;
    cursor: pointer; padding: 0; line-height: 1;
  }
  .close:hover { color: #e8e6e3; }
  .kv {
    display: flex; justify-content: space-between; align-items: center;
    padding: 0.4rem 0; border-bottom: 1px solid #2d374833; font-size: 0.85rem;
  }
  .kv .k { color: #718096; text-transform: uppercase; font-size: 0.75rem; letter-spacing: 0.05em; }
  .kv .v { color: #e8e6e3; }
  .desc { color: #a0aec0; font-size: 0.9rem; line-height: 1.5; margin-top: 0.75rem; }
  .panel-actions { margin-top: 1rem; display: flex; flex-direction: column; gap: 0.5rem; }
</style>
