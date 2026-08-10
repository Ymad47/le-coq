<script>
  import { onMount, tick } from 'svelte';
  import cytoscape from 'cytoscape';

  let entities = [];
  let relationships = [];
  let query = '';
  let loading = false;
  let error = '';
  let view = 'list'; // 'list' | 'graph'
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
        const data = await api('/api/v1/entities');
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
        api('/api/v1/entities'),
        api('/api/v1/relationships'),
      ]);
      entities = Array.isArray(ents) ? ents : ents.entities ?? [];
      relationships = Array.isArray(rels) ? rels : rels.relationships ?? [];
      view = 'graph';
      await tick();
      renderGraph();
    } catch (e) {
      error = `Chargement du graphe échoué : ${e.message}`;
    }
    loading = false;
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
    unknown: '#718096',      // gris
  };

  const EDGE_COLORS = {
    parent_of: '#c9a227',
    exports_to: '#4299e1',
    imports_from: '#4299e1',
    has_indicator: '#4a5568',
    default: '#4a5568',
  };

  let selectedNode = null;
  let graphLayout = 'cose';
  let nodeCount = 0;
  let edgeCount = 0;

  function colorForType(t) {
    if (!t) return TYPE_COLORS.unknown;
    return TYPE_COLORS[t.toLowerCase()] ?? TYPE_COLORS.unknown;
  }

  function colorForEdge(rel) {
    if (!rel) return EDGE_COLORS.default;
    return EDGE_COLORS[rel.toLowerCase()] ?? EDGE_COLORS.default;
  }

  function renderGraph() {
    if (!graphEl) return;
    cy?.destroy();

    // Entities: keep full data for tooltip/panel
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
    const nodeIds = new Set(entities.map((e) => e.id));

    // Edges: accept both old (source_id/target_id) and new (from_entity/to_entity) keys
    const edges = relationships
      .map((r) => {
        const from = r.source_id ?? r.from_entity;
        const to = r.target_id ?? r.to_entity;
        const relType = r.relation_type ?? r.relationship_type ?? r.type ?? '';
        return { from, to, relType, rawId: r.id };
      })
      .filter((e) => e.from && e.to && nodeIds.has(e.from) && nodeIds.has(e.to))
      .map((e) => ({
        data: {
          id: e.rawId,
          source: e.from,
          target: e.to,
          label: e.relType,
          relType: e.relType,
        },
      }));

    // Compute degree for sizing
    const deg = new Map();
    edges.forEach((e) => {
      deg.set(e.data.source, (deg.get(e.data.source) ?? 0) + 1);
      deg.set(e.data.target, (deg.get(e.data.target) ?? 0) + 1);
    });
    nodes.forEach((n) => {
      const d = deg.get(n.data.id) ?? 0;
      n.data.degree = d;
      n.data.size = Math.min(20 + d * 2, 60); // cap à 60px
    });

    nodeCount = nodes.length;
    edgeCount = edges.length;

    const showEdgeLabel = edgeCount < 30;

    cy = cytoscape({
      container: graphEl,
      elements: [...nodes, ...edges],
      style: [
        {
          selector: 'node',
          style: {
            label: 'data(label)',
            'background-color': (ele) => colorForType(ele.data('type')),
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
      layout: { name: graphLayout, animate: false, padding: 30 },
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

  onMount(async () => {
    try {
      const h = await api('/health');
      health = typeof h === 'string' ? h : h.status ?? 'ok';
    } catch {
      health = 'hors ligne';
    }
    // Load sources, set smart defaults
    try {
      const s = await api('/api/v1/sources');
      allSources = Array.isArray(s) ? s : [];
      // Pre-select fast/stable ones
      const wanted = new Set(DEFAULT_SOURCES);
      selectedSources = new Set(allSources.filter((x) => wanted.has(x)));
      // Fallback: take all if none of defaults found
      if (selectedSources.size === 0) selectedSources = new Set(allSources);
    } catch {
      allSources = [];
    }
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
            <h3>{selectedNode.label}</h3>
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
    width: 320px; background: #1a202c; border: 1px solid #c9a22733;
    border-radius: 10px; padding: 1rem 1.25rem; overflow-y: auto; max-height: 70vh;
  }
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
