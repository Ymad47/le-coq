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

  function renderGraph() {
    if (!graphEl) return;
    cy?.destroy();
    const nodes = entities.map((e) => ({
      data: { id: e.id, label: e.name, type: e.entity_type ?? e.type ?? 'unknown' },
    }));
    const nodeIds = new Set(entities.map((e) => e.id));
    const edges = relationships
      .filter((r) => nodeIds.has(r.source_id) && nodeIds.has(r.target_id))
      .map((r) => ({
        data: {
          id: r.id,
          source: r.source_id,
          target: r.target_id,
          label: r.relation_type ?? r.type ?? '',
        },
      }));
    cy = cytoscape({
      container: graphEl,
      elements: [...nodes, ...edges],
      style: [
        {
          selector: 'node',
          style: {
            label: 'data(label)',
            'background-color': '#c9a227',
            color: '#e8e6e3',
            'font-size': 10,
            'text-valign': 'bottom',
            'text-margin-y': 4,
            width: 22,
            height: 22,
          },
        },
        {
          selector: 'edge',
          style: {
            label: 'data(label)',
            'line-color': '#4a5568',
            'target-arrow-color': '#4a5568',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            'font-size': 8,
            color: '#a0aec0',
            width: 1.5,
          },
        },
      ],
      layout: { name: 'cose', animate: false },
    });
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
    <label class="online-toggle" title="Inclure Wikipedia + Wikidata (ingestion auto en cache)">
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
        <article class="card">
          <div class="card-head">
            <span class="badge">{typeBadge(e.entity_type ?? e.type)} {e.entity_type ?? e.type ?? 'entité'}</span>
            {#if e.country ?? e.properties?.country}<span class="country">{e.country ?? e.properties?.country}</span>{/if}
          </div>
          <h3>{e.name}</h3>
          {#if desc}<p>{desc}</p>{/if}
        </article>
      {:else}
        {#if !loading}<p class="empty">Aucune entité. Lancez une ingestion via le CLI.</p>{/if}
      {/each}
    </section>
  {:else}
    <div class="graph-wrap" bind:this={graphEl}></div>
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
  .graph-wrap {
    height: 70vh; background: #141822; border: 1px solid #2d3748; border-radius: 10px;
  }
</style>
