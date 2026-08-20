<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Entity, View, MapCountryInfo, GraphNode } from './lib/types';
  import * as api from './lib/api';
  import ListView from './components/ListView.svelte';
  import GraphView from './components/GraphView.svelte';
  import MapView from './components/MapView.svelte';
  import ArticleDrawer from './components/ArticleDrawer.svelte';

  // === State ===
  let entities: Entity[] = [];
  let relationships: any[] = [];
  let query = '';
  let loading = false;
  let error = '';
  let view: View = 'list';
  let focusHubId: string | null = null;
  let health = '';

  let ingestOnline = true;
  let newItems = 0;
  let searchErrors: string[] = [];
  let searching = false;
  let allSources: string[] = [];
  let selectedSources = new Set<string>();

  const DEFAULT_SOURCES = ['wikipedia', 'wikidata', 'gleif', 'eurostat'];

  // === Component refs ===
  let listView: ListView;
  let graphView: GraphView;
  let mapView: MapView;

  // === Article drawer state ===
  let articleEntity: Entity | null = null;

  // === Map state ===
  let mapInfo: MapCountryInfo | null = null;

  // === Actions ===

  async function search(): Promise<void> {
    if (!query.trim()) {
      loading = true;
      try {
        entities = await api.listEntities(1000);
        newItems = 0;
        searchErrors = [];
      } catch (e: any) {
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
      const data = await api.smartSearch(query, ingestOnline, selectedSources);
      entities = data.results.map((r) => ({
        id: r.id,
        name: r.name,
        entity_type: r.entity_type,
        properties: r.properties,
        description: r.properties?.description ?? null,
        country: r.properties?.country ?? null,
      }));
      newItems = data.new_items ?? 0;
      searchErrors = data.errors ?? [];
    } catch (e: any) {
      error = `Recherche échouée : ${e.message}`;
    }
    loading = false;
    searching = false;
  }

  async function loadGraph(): Promise<void> {
    loading = true;
    error = '';
    try {
      const [ents, rels] = await Promise.all([api.listEntities(1000), api.listRelationships(5000)]);
      entities = ents;
      relationships = rels;
      view = 'graph';
      await tick();
      await graphView?.renderGraph();
    } catch (e: any) {
      error = `Vue Graphe échouée : ${e.message}`;
    }
    loading = false;
  }

  async function loadFocus(skipSearch = false, directHubId: string | null = null): Promise<void> {
    if (!query.trim() && !directHubId) {
      error = "Entrez une recherche d'abord pour utiliser Vue Focus.";
      return;
    }
    loading = true;
    searching = !skipSearch;
    error = '';

    try {
      const qLower = query.trim().toLowerCase();
      let hub: Entity | null = null;

      if (!skipSearch) {
        const data = await api.smartSearch(query, ingestOnline, selectedSources);
        newItems = data.new_items ?? 0;
        searchErrors = data.errors ?? [];
        const results = data.results;
        hub =
          results.find((r) => (r.name ?? '').toLowerCase() === qLower) ??
          results.find((r) => (r.name ?? '').toLowerCase().startsWith(qLower)) ??
          results[0];
      }

      const [ents, rels] = await Promise.all([api.listEntities(1000), api.listRelationships(5000)]);
      entities = ents;
      relationships = rels;

      if (directHubId) {
        focusHubId = directHubId;
      } else {
        if (!hub) {
          hub =
            entities.find((e) => (e.name ?? '').toLowerCase() === qLower) ??
            entities.find((e) => (e.name ?? '').toLowerCase().startsWith(qLower)) ??
            entities.find((e) => (e.name ?? '').toLowerCase().includes(qLower)) ??
            null;
        }
        focusHubId = hub?.id ?? null;
      }

      view = 'focus';
      await tick();
      await graphView?.renderGraph();
      if (focusHubId) {
        // Wait for layout, then center
        setTimeout(() => graphView?.focusHub(focusHubId!), 800);
      }
    } catch (e: any) {
      error = `Vue Focus échouée : ${e.message}`;
    }
    loading = false;
    searching = false;
  }

  async function loadMap(): Promise<void> {
    loading = true;
    error = '';
    view = 'map';
    try {
      entities = await api.listEntities(1000);
      await tick();
      await mapView?.renderMap();
    } catch (e: any) {
      error = `Vue carte échouée : ${e.message}`;
    }
    loading = false;
  }

  function handleKey(e: KeyboardEvent): void {
    if (e.key === 'Enter') search();
  }

  async function loadSources(): Promise<void> {
    try {
      const s = await api.listSources();
      allSources = Array.isArray(s) ? s : [];
      const wanted = new Set(DEFAULT_SOURCES);
      selectedSources = new Set(allSources.filter((x) => wanted.has(x)));
      if (selectedSources.size === 0) selectedSources = new Set(allSources);
    } catch {
      allSources = [];
    }
  }

  async function reloadPlugins(): Promise<void> {
    try {
      const r = await api.reloadPlugins();
      await loadSources();
      if (r.failed?.length) {
        error = `Plugins en échec: ${r.failed.map((f) => f.name).join(', ')}`;
      }
    } catch (e: any) {
      error = `Reload plugins échoué : ${e.message}`;
    }
  }

  // === Event handlers from children ===

  function handleOpenArticle(e: CustomEvent<Entity>): void {
    articleEntity = e.detail;
  }

  function handleCloseArticle(): void {
    articleEntity = null;
  }

  function handleOpenInFocus(e: CustomEvent<{ id: string; name: string }>): void {
    query = e.detail.name;
    articleEntity = null;
    view = 'graph';
    loadFocus(true, e.detail.id);
  }

  function handleNodeSelected(e: CustomEvent<GraphNode>): void {
    // Optional: show feedback
  }

  function handleExploreFromNode(e: CustomEvent<{ query: string }>): void {
    query = e.detail.query;
    ingestOnline = true;
    view = 'list';
    search();
  }

  async function handleEnrichFromNode(e: CustomEvent<{ query: string }>): Promise<void> {
    query = e.detail.query;
    ingestOnline = true;
    await search();
    if (view === 'graph' || view === 'focus') await loadGraph();
  }

  function handleCountrySelected(e: CustomEvent<MapCountryInfo>): void {
    mapInfo = e.detail;
  }

  function handleSearchCountry(query: string): void {
    query = query;
    mapInfo = null;
    view = 'list';
    search();
  }

  async function handleIngestCountry(query: string): Promise<void> {
    query = query;
    ingestOnline = true;
    mapInfo = null;
    view = 'list';
    await search();
  }

  // === Init ===

  onMount(async () => {
    try {
      const h = await api.checkHealth();
      health = typeof h === 'string' ? h : (h as any).status ?? 'ok';
    } catch {
      health = 'hors ligne';
    }
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
    <button
      class="secondary focus-btn"
      on:click={() => loadFocus()}
      disabled={loading || !query.trim()}
      title="Cas 10-min : smart search + graphe centré sur l'entité"
    >
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
            selectedSources = selectedSources;
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
    <ListView {entities} {loading} on:openArticle={handleOpenArticle} />
  {:else if view === 'map'}
    <MapView bind:this={mapView} {entities} on:countrySelected={handleCountrySelected} />

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
            <button class="secondary small" on:click={() => { if (mapInfo) handleSearchCountry(mapInfo.name); }}>
              🔎 Rechercher "{mapInfo.name}"
            </button>
          </div>
        {:else}
          <div class="drawer-actions">
            <button class="secondary small" on:click={() => { if (mapInfo) handleIngestCountry(mapInfo.name); }}>
              🌐 Lancer une recherche en ligne sur "{mapInfo.name}"
            </button>
          </div>
        {/if}
      </aside>
    {/if}
  {:else}
    <GraphView
      bind:this={graphView}
      {entities}
      {relationships}
      {focusHubId}
      initialLayout={view === 'focus' ? 'concentric' : 'cose'}
      on:nodeSelected={handleNodeSelected}
      on:exploreFromNode={handleExploreFromNode}
      on:enrichFromNode={handleEnrichFromNode}
    />
  {/if}
</main>

<!-- Article drawer: side panel with entity details + relations -->
<ArticleDrawer
  entity={articleEntity}
  on:close={handleCloseArticle}
  on:openInFocus={handleOpenInFocus}
/>

<style>
  /* Design tokens */
  :global(:root) {
    --bg: #0a0a0f;
    --teal: #2dd4bf;
    --teal-light: #5eead4;
    --teal-dark: #0d9488;
    --text: #e2e8f0;
    --text-dim: #94a3b8;
    --text-dimmer: #64748b;
    --glass-bg: rgba(255, 255, 255, 0.03);
    --glass-bg-hover: rgba(255, 255, 255, 0.05);
    --glass-border: rgba(255, 255, 255, 0.06);
    --glass-border-strong: rgba(255, 255, 255, 0.1);
  }

  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    background: var(--bg);
    color: var(--text);
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', system-ui, sans-serif;
    font-size: 15px;
    line-height: 1.5;
    margin: 0;
    -webkit-font-smoothing: antialiased;
  }

  :global(body)::before {
    content: '';
    position: fixed;
    top: -50%;
    left: -50%;
    width: 200%;
    height: 200%;
    background: radial-gradient(circle at 30% 40%, rgba(45, 212, 191, 0.08), transparent 40%);
    filter: blur(80px);
    pointer-events: none;
    z-index: -2;
  }

  :global(body)::after {
    content: '';
    position: fixed;
    bottom: -30%;
    right: -20%;
    width: 80%;
    height: 80%;
    background: radial-gradient(circle, rgba(45, 212, 191, 0.05), transparent 50%);
    filter: blur(60px);
    pointer-events: none;
    z-index: -1;
  }

  main {
    max-width: 1400px;
    margin: 0 auto;
    padding: 2rem 1.5rem;
    min-height: 100vh;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 2rem;
  }

  .brand h1 {
    font-size: 1.75rem;
    margin: 0;
    font-weight: 700;
    background: linear-gradient(135deg, #fff, var(--teal-light));
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .subtitle {
    color: var(--text-dim);
    margin: 0.25rem 0 0 0;
    font-size: 0.9rem;
  }

  .health {
    color: var(--teal);
    font-size: 0.85rem;
    padding: 0.35rem 0.75rem;
    background: rgba(45, 212, 191, 0.08);
    border-radius: 999px;
    border: 1px solid rgba(45, 212, 191, 0.2);
  }

  .health.down {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.08);
    border-color: rgba(239, 68, 68, 0.2);
  }

  .toolbar {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    margin-bottom: 1.5rem;
    padding: 0.75rem;
    background: var(--glass-bg);
    backdrop-filter: blur(20px) saturate(180%);
    border-radius: 16px;
    border: 1px solid var(--glass-border);
  }

  .toolbar input[type='text'] {
    flex: 1;
    min-width: 200px;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--glass-border);
    border-radius: 12px;
    padding: 0.65rem 1rem;
    color: var(--text);
    font-size: 0.95rem;
    outline: none;
    transition: all 0.2s;
  }

  .toolbar input[type='text']:focus {
    border-color: var(--teal);
    box-shadow: 0 0 0 3px rgba(45, 212, 191, 0.1);
  }

  .online-toggle {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--text-dim);
    font-size: 0.85rem;
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
  }

  .online-toggle input[type='checkbox'] {
    accent-color: var(--teal);
  }

  button {
    background: var(--teal);
    color: var(--bg);
    border: none;
    border-radius: 10px;
    padding: 0.65rem 1.25rem;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
  }

  button:hover:not(:disabled) {
    background: var(--teal-light);
    transform: translateY(-1px);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  button.secondary {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text);
    border: 1px solid var(--glass-border);
  }

  button.secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    border-color: var(--glass-border-strong);
  }

  button.small {
    padding: 0.4rem 0.85rem;
    font-size: 0.8rem;
  }

  .source-chips {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
    margin-bottom: 1rem;
    padding: 0.5rem 0.75rem;
    background: rgba(255, 255, 255, 0.02);
    border-radius: 12px;
    border: 1px solid var(--glass-border);
  }

  .chips-label {
    color: var(--text-dim);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .chip {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--glass-border);
    color: var(--text-dim);
    padding: 0.35rem 0.85rem;
    border-radius: 999px;
    font-size: 0.8rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .chip:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text);
  }

  .chip.active {
    background: rgba(45, 212, 191, 0.15);
    border-color: var(--teal);
    color: var(--teal);
  }

  .chip.reload {
    margin-left: auto;
    background: rgba(255, 255, 255, 0.04);
  }

  .error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #fca5a5;
    padding: 0.75rem 1rem;
    border-radius: 12px;
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }

  .warn {
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.3);
    color: #fcd34d;
    padding: 0.75rem 1rem;
    border-radius: 12px;
    margin-bottom: 1rem;
    font-size: 0.85rem;
  }

  .info {
    background: rgba(45, 212, 191, 0.08);
    border: 1px solid rgba(45, 212, 191, 0.25);
    color: var(--teal-light);
    padding: 0.75rem 1rem;
    border-radius: 12px;
    margin-bottom: 1rem;
    font-size: 0.85rem;
  }

  .country-panel {
    position: fixed;
    left: 1.5rem;
    top: 50%;
    transform: translateY(-50%);
    width: 360px;
    max-height: 80vh;
    overflow-y: auto;
    background: rgba(10, 10, 15, 0.95);
    backdrop-filter: blur(40px) saturate(180%);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 16px;
    padding: 1.25rem;
    z-index: 50;
    box-shadow: 10px 0 40px rgba(0, 0, 0, 0.6);
  }

  .country-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 0.75rem;
  }

  .country-head h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text);
  }

  .country-head .close {
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 1.3rem;
    cursor: pointer;
    padding: 0;
    line-height: 1;
  }

  .country-head .close:hover {
    color: var(--text);
  }

  .country-meta {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0 0 0.75rem 0;
  }

  .country-list {
    list-style: none;
    margin: 0 0 1rem 0;
    padding: 0;
  }

  .country-item {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.4rem 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.85rem;
    color: var(--text-dim);
  }

  .country-item-name {
    flex: 1;
    color: var(--text);
  }

  .country-item.dim {
    color: var(--text-dimmer);
    font-style: italic;
  }

  .drawer-actions {
    display: flex;
    gap: 0.5rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--glass-border);
  }

  /* Mobile responsive overrides — appended at end */
  @media (max-width: 820px) {
    main {
      padding: 1rem 0.75rem;
    }

    header {
      flex-direction: column;
      gap: 0.75rem;
      margin-bottom: 1.5rem;
    }

    .brand h1 {
      font-size: 1.4rem;
    }

    :global(body) {
      font-size: 14px;
    }

    .toolbar {
      flex-wrap: wrap;
    }

    .source-chips {
      flex-wrap: nowrap;
      overflow-x: auto;
      scrollbar-width: none;
    }

    .source-chips::-webkit-scrollbar {
      display: none;
    }

    .country-panel {
      left: 0.5rem;
      right: 0.5rem;
      bottom: 0.5rem;
      top: auto;
      transform: none;
      width: auto;
      max-height: 40vh;
    }
  }

  @media (max-width: 400px) {
    :global(body) {
      font-size: 13px;
    }

    .brand h1 {
      font-size: 1.2rem;
    }
  }
</style>
