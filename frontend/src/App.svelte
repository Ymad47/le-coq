<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Entity, Relationship, View, MapCountryInfo } from './lib/types';
  import * as api from './lib/api';
  import { readRoute, writeRoute } from './lib/route';
  import { typeBadge } from './lib/utils';
  import ListView from './components/ListView.svelte';
  import GraphView from './components/GraphView.svelte';
  import MapView from './components/MapView.svelte';
  import ArticleDrawer from './components/ArticleDrawer.svelte';

  // === State ===
  let query = '';
  let view: View = 'list';

  /** Search results (list view + seeds of the graph view). */
  let entities: Entity[] = [];
  /** Subgraph currently shown in graph/focus views. */
  let graphEntities: Entity[] = [];
  let graphRels: Relationship[] = [];
  let graphTruncated = false;
  let graphScope = '';
  let focusHubId: string | null = null;
  /** Whole-database sample for the map (independent from search results). */
  let mapEntities: Entity[] = [];

  let loading = false;
  let searching = false;
  let error = '';
  let searchErrors: string[] = [];
  let newItems = 0;

  let healthOk: boolean | null = null;
  let healthText = '';

  let ingestOnline = true;
  let allSources: string[] = [];
  let selectedSources = new Set<string>();
  const DEFAULT_SOURCES = ['wikipedia', 'wikidata', 'gleif', 'eurostat'];

  /** Entity types hidden from the list (empty = show all). */
  let hiddenTypes = new Set<string>();

  let articleEntity: Entity | null = null;
  let mapInfo: MapCountryInfo | null = null;

  let graphView: GraphView;
  let mapView: MapView;
  let searchInput: HTMLInputElement;

  const TABS: { id: View; label: string; key: string; hint: string }[] = [
    { id: 'list', label: 'Liste', key: '1', hint: 'Résultats groupés par pays ou par type' },
    { id: 'graph', label: 'Graphe', key: '2', hint: 'Résultats et leurs voisins directs' },
    { id: 'focus', label: 'Focus', key: '3', hint: "Graphe centré sur l'entité recherchée (2 sauts)" },
    { id: 'map', label: 'Carte', key: '4', hint: "Densité d'entités par pays (toute la base)" },
  ];

  // Each load gets a token; responses from an outdated request are dropped.
  let requestToken = 0;

  // === Derived ===
  const typeOf = (e: Entity) => e.entity_type ?? e.type ?? 'inconnu';

  $: typeCounts = (() => {
    const m = new Map<string, number>();
    for (const e of entities) m.set(typeOf(e), (m.get(typeOf(e)) ?? 0) + 1);
    return [...m.entries()].sort((a, b) => b[1] - a[1]);
  })();

  $: visibleEntities = hiddenTypes.size ? entities.filter((e) => !hiddenTypes.has(typeOf(e))) : entities;

  // === Data loading ===

  function normalize(r: any): Entity {
    return {
      id: r.id,
      name: r.name,
      entity_type: r.entity_type ?? r.type,
      properties: r.properties,
      description: r.properties?.description ?? null,
      country: r.properties?.country ?? null,
    };
  }

  /** Runs the search for `query`. Returns false if superseded by a newer request. */
  async function runSearch(online = ingestOnline): Promise<boolean> {
    const token = ++requestToken;
    loading = true;
    searching = online && !!query.trim();
    error = '';
    searchErrors = [];
    newItems = 0;
    try {
      if (!query.trim()) {
        const list = await api.listEntities(1000);
        if (token !== requestToken) return false;
        entities = list.map(normalize);
      } else {
        const data = await api.smartSearch(query, online, selectedSources);
        if (token !== requestToken) return false;
        entities = data.results.map(normalize);
        newItems = data.new_items ?? 0;
        searchErrors = data.errors ?? [];
      }
      return true;
    } catch (e: any) {
      if (token === requestToken) error = `Recherche échouée : ${e.message}`;
      return false;
    } finally {
      if (token === requestToken) {
        loading = false;
        searching = false;
      }
    }
  }

  /** Graph of the current results + their direct neighbors. */
  async function loadGraph(): Promise<void> {
    const token = ++requestToken;
    loading = true;
    error = '';
    try {
      if (entities.length === 0) {
        graphEntities = [];
        graphRels = [];
        graphTruncated = false;
        graphScope = 'aucun résultat à afficher';
      } else {
        const seeds = entities.map((e) => e.id);
        const hood = await api.getNeighborhood(seeds);
        if (token !== requestToken) return;
        graphEntities = hood.entities.map(normalize);
        graphRels = hood.relationships;
        graphTruncated = hood.truncated || seeds.length > 500;
        graphScope = query.trim()
          ? `voisinage de ${Math.min(seeds.length, 500)} résultat${seeds.length > 1 ? 's' : ''}`
          : `voisinage des ${Math.min(seeds.length, 500)} entités les plus récentes`;
      }
      focusHubId = null;
      await tick();
      await graphView?.renderGraph();
    } catch (e: any) {
      if (token === requestToken) error = `Vue Graphe échouée : ${e.message}`;
    } finally {
      if (token === requestToken) loading = false;
    }
  }

  function pickHub(list: Entity[], q: string): Entity | null {
    const ql = q.trim().toLowerCase();
    if (!ql) return list[0] ?? null;
    return (
      list.find((r) => (r.name ?? '').toLowerCase() === ql) ??
      list.find((r) => (r.name ?? '').toLowerCase().startsWith(ql)) ??
      list.find((r) => (r.name ?? '').toLowerCase().includes(ql)) ??
      list[0] ??
      null
    );
  }

  /** Graph centered on one entity, 2 hops deep. */
  async function loadFocus(directHubId: string | null = null): Promise<void> {
    const hubId = directHubId ?? pickHub(entities, query)?.id ?? null;
    if (!hubId) {
      error = 'Aucune entité à centrer : lancez une recherche qui renvoie au moins un résultat.';
      graphEntities = [];
      graphRels = [];
      return;
    }
    const token = ++requestToken;
    loading = true;
    error = '';
    try {
      const hop1 = await api.getNeighborhood([hubId]);
      if (token !== requestToken) return;
      const ring = hop1.entities.map((e) => e.id).slice(0, 500);
      const hop2 = ring.length > 1 ? await api.getNeighborhood(ring) : hop1;
      if (token !== requestToken) return;
      graphEntities = hop2.entities.map(normalize);
      graphRels = hop2.relationships;
      graphTruncated = hop1.truncated || hop2.truncated;
      focusHubId = hubId;
      const hubName = graphEntities.find((e) => e.id === hubId)?.name ?? hubId;
      graphScope = `centré sur ${hubName} · 2 sauts`;
      await tick();
      await graphView?.renderGraph();
      // Wait for the layout to settle before centering
      setTimeout(() => focusHubId && graphView?.focusHub(focusHubId), 600);
    } catch (e: any) {
      if (token === requestToken) error = `Vue Focus échouée : ${e.message}`;
    } finally {
      if (token === requestToken) loading = false;
    }
  }

  async function loadMap(): Promise<void> {
    const token = ++requestToken;
    loading = true;
    error = '';
    try {
      const list = await api.listEntities(1000);
      if (token !== requestToken) return;
      mapEntities = list.map(normalize);
      await tick();
      await mapView?.renderMap();
    } catch (e: any) {
      if (token === requestToken) error = `Vue Carte échouée : ${e.message}`;
    } finally {
      if (token === requestToken) loading = false;
    }
  }

  /** Loads whatever the current view needs. */
  async function activate(v: View, opts: { search?: boolean; online?: boolean } = {}): Promise<void> {
    view = v;
    mapInfo = null;
    if (v === 'map') return loadMap();
    if (opts.search) {
      const ok = await runSearch(opts.online ?? ingestOnline);
      if (!ok) return;
    }
    if (v === 'graph') return loadGraph();
    if (v === 'focus') return loadFocus();
  }

  // === User actions ===

  function setView(v: View): void {
    if (v === view) return;
    writeRoute({ view: v, query });
    activate(v);
  }

  function submitSearch(): void {
    // Searching from the map shows results in the list
    const target: View = view === 'map' ? 'list' : view;
    writeRoute({ view: target, query });
    activate(target, { search: true });
  }

  function searchFor(q: string, opts: { online?: boolean; view?: View } = {}): void {
    query = q;
    if (opts.online) ingestOnline = true;
    const target = opts.view ?? 'list';
    writeRoute({ view: target, query });
    activate(target, { search: true, online: opts.online ?? ingestOnline });
  }

  function toggleSource(s: string): void {
    if (selectedSources.has(s)) selectedSources.delete(s);
    else selectedSources.add(s);
    selectedSources = selectedSources;
  }

  function toggleType(t: string): void {
    if (hiddenTypes.has(t)) hiddenTypes.delete(t);
    else hiddenTypes.add(t);
    hiddenTypes = hiddenTypes;
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
      error = r.failed?.length ? `Plugins en échec : ${r.failed.map((f) => f.name).join(', ')}` : '';
    } catch (e: any) {
      error = `Rechargement des plugins échoué : ${e.message}`;
    }
  }

  async function checkHealth(): Promise<void> {
    try {
      healthText = await api.checkHealth();
      healthOk = true;
    } catch (e: any) {
      healthText = e.message ?? 'injoignable';
      healthOk = false;
    }
  }

  // === Child events ===

  function handleOpenInFocus(e: CustomEvent<{ id: string; name: string }>): void {
    articleEntity = null;
    query = e.detail.name;
    view = 'focus';
    writeRoute({ view: 'focus', query });
    loadFocus(e.detail.id);
  }

  function handleExploreFromNode(e: CustomEvent<{ query: string }>): void {
    searchFor(e.detail.query, { online: true, view: 'list' });
  }

  function handleEnrichFromNode(e: CustomEvent<{ query: string }>): void {
    // Ingest the stub online, then redraw the same view around it
    searchFor(e.detail.query, { online: true, view: view === 'focus' ? 'focus' : 'graph' });
  }

  // === Keyboard ===

  function isTyping(t: EventTarget | null): boolean {
    const el = t as HTMLElement | null;
    return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable);
  }

  function handleGlobalKey(e: KeyboardEvent): void {
    if ((e.key === 'k' && (e.ctrlKey || e.metaKey)) || (e.key === '/' && !isTyping(e.target))) {
      e.preventDefault();
      searchInput?.focus();
      searchInput?.select();
      return;
    }
    if (isTyping(e.target) || e.ctrlKey || e.metaKey || e.altKey || articleEntity) return;
    const tab = TABS.find((t) => t.key === e.key);
    if (tab) setView(tab.id);
  }

  function handleSearchKey(e: KeyboardEvent): void {
    if (e.key === 'Enter') submitSearch();
    if (e.key === 'Escape') searchInput.blur();
  }

  // === Init ===

  function applyRoute(): void {
    const r = readRoute();
    query = r.query;
    // Restored links only search the local cache: opening a URL must not
    // silently trigger outbound requests to external sources.
    activate(r.view, { search: r.view !== 'map', online: false });
  }

  onMount(() => {
    checkHealth();
    loadSources();
    applyRoute();
    window.addEventListener('popstate', applyRoute);
    return () => window.removeEventListener('popstate', applyRoute);
  });
</script>

<svelte:window on:keydown={handleGlobalKey} />

<div class="app">
  <header class="topbar">
    <a class="brand" href="#/list" on:click|preventDefault={() => searchFor('', { view: 'list' })}>
      <span class="logo">🐓</span>
      <span class="brand-text">
        <span class="brand-name">Le Coq</span>
        <span class="brand-sub">Renseignement géopolitique</span>
      </span>
    </a>

    <div class="search">
      <span class="search-icon" aria-hidden="true">⌕</span>
      <input
        bind:this={searchInput}
        type="search"
        bind:value={query}
        on:keydown={handleSearchKey}
        placeholder="Rechercher une entité, une organisation, un pays…"
        aria-label="Recherche"
      />
      <kbd class="search-kbd" title="Raccourci clavier">Ctrl K</kbd>
    </div>

    <label class="online" title="Interroger aussi les sources externes et ingérer les résultats">
      <input type="checkbox" bind:checked={ingestOnline} />
      <span>Sources externes</span>
    </label>

    <button on:click={submitSearch} disabled={loading}>
      {searching ? 'Recherche…' : 'Rechercher'}
    </button>

    <span
      class="health"
      class:ok={healthOk === true}
      class:down={healthOk === false}
      title={healthText}
    >
      ● {healthOk === null ? '…' : healthOk ? 'API en ligne' : 'API hors ligne'}
    </span>
  </header>

  <nav class="tabs" aria-label="Vues">
    {#each TABS as t}
      <button
        class="tab"
        class:active={view === t.id}
        aria-current={view === t.id ? 'page' : undefined}
        title="{t.hint} (touche {t.key})"
        on:click={() => setView(t.id)}
      >
        {t.label}
        <kbd>{t.key}</kbd>
      </button>
    {/each}
  </nav>

  <div class="body">
    <aside class="sidebar">
      <details open>
        <summary>Sources externes</summary>
        <div class="side-section" class:muted={!ingestOnline}>
          {#if !ingestOnline}
            <p class="hint">Désactivées : seule la base locale est interrogée.</p>
          {/if}
          <div class="chips">
            {#each allSources as s}
              <button
                class="chip"
                class:active={selectedSources.has(s)}
                aria-pressed={selectedSources.has(s)}
                disabled={!ingestOnline}
                on:click={() => toggleSource(s)}
              >
                {s}
              </button>
            {:else}
              <p class="hint">Aucun plugin chargé.</p>
            {/each}
          </div>
          <button class="ghost small reload" on:click={reloadPlugins} title="Recharger les plugins Python">
            ↻ Recharger les plugins
          </button>
        </div>
      </details>

      {#if view === 'list' && typeCounts.length}
        <details open>
          <summary>Types</summary>
          <div class="side-section">
            {#each typeCounts as [t, n]}
              <label class="type-row">
                <input type="checkbox" checked={!hiddenTypes.has(t)} on:change={() => toggleType(t)} />
                <span class="type-name">{typeBadge(t)} {t}</span>
                <span class="type-count">{n}</span>
              </label>
            {/each}
            {#if hiddenTypes.size}
              <button class="ghost small" on:click={() => (hiddenTypes = new Set())}>Tout afficher</button>
            {/if}
          </div>
        </details>
      {/if}

      <div class="shortcuts">
        <kbd>/</kbd> rechercher · <kbd>1</kbd>–<kbd>4</kbd> vues · <kbd>Échap</kbd> fermer
      </div>
    </aside>

    <main class="content">
      <div class="messages" aria-live="polite">
        {#if error}
          <div class="alert error">{error}</div>
        {/if}
        {#if searching}
          <div class="alert info">
            Interrogation de {selectedSources.size} source{selectedSources.size > 1 ? 's' : ''} externe{selectedSources.size > 1 ? 's' : ''}…
            cela peut prendre jusqu'à 2 minutes.
          </div>
        {/if}
        {#if searchErrors.length}
          <div class="alert warn">⚠️ {searchErrors.join(' · ')}</div>
        {/if}
        {#if newItems > 0 && !error}
          <div class="alert info">
            {newItems} élément{newItems !== 1 ? 's' : ''} ingéré{newItems !== 1 ? 's' : ''} depuis les sources externes
          </div>
        {/if}
      </div>

      {#if view === 'list'}
        <ListView
          entities={visibleEntities}
          {loading}
          {query}
          filtered={hiddenTypes.size > 0 && entities.length > 0}
          on:openArticle={(e) => (articleEntity = e.detail)}
          on:suggest={(e) => searchFor(e.detail.query, { online: true })}
          on:clearFilters={() => (hiddenTypes = new Set())}
        />
      {:else if view === 'map'}
        <MapView bind:this={mapView} entities={mapEntities} on:countrySelected={(e) => (mapInfo = e.detail)} />

        {#if mapInfo}
          <aside class="country-panel panel">
            <div class="country-head">
              <h3>🌍 {mapInfo.name}</h3>
              <button class="ghost icon" on:click={() => (mapInfo = null)} aria-label="Fermer">×</button>
            </div>
            <p class="country-meta">
              {mapInfo.entities.length > 0
                ? `${mapInfo.entities.length} entité${mapInfo.entities.length > 1 ? 's' : ''} dans la base`
                : 'Aucune entité dans la base'}
            </p>
            {#if mapInfo.entities.length > 0}
              <ul class="country-list">
                {#each mapInfo.entities.slice(0, 30) as e (e.id)}
                  <li>
                    <button class="country-item" on:click={() => (articleEntity = e)}>
                      <span class="badge">{typeOf(e)}</span>
                      <span class="country-item-name">{e.name}</span>
                    </button>
                  </li>
                {/each}
                {#if mapInfo.entities.length > 30}
                  <li class="dim">… et {mapInfo.entities.length - 30} autres</li>
                {/if}
              </ul>
            {/if}
            <div class="country-actions">
              <button class="secondary small" on:click={() => mapInfo && searchFor(mapInfo.name)}>
                Rechercher « {mapInfo.name} »
              </button>
              <button class="secondary small" on:click={() => mapInfo && searchFor(mapInfo.name, { online: true })}>
                🌐 Chercher en ligne
              </button>
            </div>
          </aside>
        {/if}
      {:else}
        {#key view}
          <GraphView
            bind:this={graphView}
            entities={graphEntities}
            relationships={graphRels}
            {focusHubId}
            truncated={graphTruncated}
            scopeLabel={graphScope}
            initialLayout={view === 'focus' ? 'concentric' : 'cose'}
            on:exploreFromNode={handleExploreFromNode}
            on:enrichFromNode={handleEnrichFromNode}
          />
        {/key}
      {/if}
    </main>
  </div>
</div>

<ArticleDrawer entity={articleEntity} on:close={() => (articleEntity = null)} on:openInFocus={handleOpenInFocus} />

<style>
  .app {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  /* ---------- Top bar ---------- */

  .topbar {
    position: sticky;
    top: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-height: var(--topbar-h);
    padding: 0.6rem 1.25rem;
    background: rgba(10, 10, 15, 0.85);
    backdrop-filter: blur(20px) saturate(180%);
    border-bottom: 1px solid var(--border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    text-decoration: none;
    color: inherit;
    margin-right: 0.5rem;
  }

  .logo {
    font-size: 1.6rem;
  }

  .brand-text {
    display: flex;
    flex-direction: column;
    line-height: 1.15;
  }

  .brand-name {
    font-weight: 700;
    font-size: 1.05rem;
    background: linear-gradient(135deg, #fff, var(--teal-light));
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .brand-sub {
    font-size: 0.72rem;
    color: var(--text-dimmer);
  }

  .search {
    position: relative;
    flex: 1;
    max-width: 640px;
    display: flex;
    align-items: center;
  }

  .search input {
    width: 100%;
    padding-left: 2rem;
    padding-right: 4rem;
  }

  .search-icon {
    position: absolute;
    left: 0.7rem;
    color: var(--text-dimmer);
    pointer-events: none;
  }

  .search-kbd {
    position: absolute;
    right: 0.6rem;
    pointer-events: none;
  }

  .online {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.82rem;
    color: var(--text-dim);
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
  }

  .health {
    margin-left: auto;
    font-size: 0.78rem;
    padding: 0.25rem 0.65rem;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--text-dim);
    white-space: nowrap;
  }

  .health.ok {
    color: var(--teal);
    background: rgba(45, 212, 191, 0.08);
    border-color: rgba(45, 212, 191, 0.25);
  }

  .health.down {
    color: var(--danger);
    background: var(--danger-soft);
    border-color: rgba(239, 68, 68, 0.3);
  }

  /* ---------- Tabs ---------- */

  .tabs {
    display: flex;
    gap: 0.25rem;
    padding: 0 1.25rem;
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    background: none;
    color: var(--text-dim);
    font-weight: 500;
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    padding: 0.7rem 0.9rem;
  }

  .tab:hover:not(:disabled) {
    background: none;
    color: var(--text);
  }

  .tab.active {
    color: var(--teal);
    border-bottom-color: var(--teal);
  }

  /* ---------- Body: sidebar + content ---------- */

  .body {
    flex: 1;
    display: grid;
    grid-template-columns: var(--sidebar-w) minmax(0, 1fr);
    gap: 1.25rem;
    padding: 1.25rem;
  }

  .sidebar {
    position: sticky;
    top: calc(var(--topbar-h) + 1rem);
    align-self: start;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .sidebar details {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .sidebar summary {
    cursor: pointer;
    padding: 0.6rem 0.85rem;
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-dim);
    list-style: none;
  }

  .sidebar summary::-webkit-details-marker {
    display: none;
  }

  .sidebar summary::after {
    content: '▾';
    float: right;
    transition: transform 0.15s;
  }

  .sidebar details:not([open]) summary::after {
    transform: rotate(-90deg);
  }

  .side-section {
    padding: 0 0.85rem 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .side-section.muted .chips {
    opacity: 0.55;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .hint {
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-dimmer);
  }

  .reload {
    align-self: flex-start;
  }

  .type-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .type-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .type-count {
    color: var(--text-dimmer);
    font-variant-numeric: tabular-nums;
  }

  .shortcuts {
    font-size: 0.72rem;
    color: var(--text-dimmer);
    padding: 0 0.25rem;
    line-height: 1.9;
  }

  .content {
    min-width: 0;
  }

  .messages {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .messages:empty {
    display: none;
  }

  /* ---------- Map country panel ---------- */

  .country-panel {
    position: fixed;
    right: 1.25rem;
    top: calc(var(--topbar-h) + 4rem);
    width: 340px;
    max-height: calc(100vh - var(--topbar-h) - 6rem);
    overflow-y: auto;
    padding: 1rem 1.1rem;
    z-index: 30;
    background: rgba(10, 10, 15, 0.96);
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.6);
  }

  .country-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .country-head h3 {
    margin: 0;
    font-size: 1.05rem;
  }

  .country-meta {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0.25rem 0 0.75rem;
  }

  .country-list {
    list-style: none;
    margin: 0 0 0.75rem;
    padding: 0;
  }

  .country-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    white-space: normal;
    background: none;
    color: var(--text);
    font-weight: 400;
    font-size: 0.85rem;
    border: none;
    border-bottom: 1px solid var(--border);
    border-radius: 0;
    padding: 0.4rem 0.25rem;
  }

  .country-item:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .country-item-name {
    flex: 1;
  }

  .dim {
    color: var(--text-dimmer);
    font-style: italic;
    font-size: 0.82rem;
    padding: 0.4rem 0.25rem;
  }

  .country-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border);
  }

  /* ---------- Responsive ---------- */

  @media (max-width: 1000px) {
    .brand-sub,
    .search-kbd {
      display: none;
    }
  }

  @media (max-width: 820px) {
    .topbar {
      flex-wrap: wrap;
      padding: 0.6rem 0.75rem;
    }

    .search {
      order: 10;
      flex-basis: 100%;
      max-width: none;
    }

    .tabs {
      padding: 0 0.75rem;
    }

    .body {
      grid-template-columns: 1fr;
      padding: 0.75rem;
    }

    .sidebar {
      position: static;
    }

    .shortcuts {
      display: none;
    }

    .country-panel {
      left: 0.5rem;
      right: 0.5rem;
      top: auto;
      bottom: 0.5rem;
      width: auto;
      max-height: 45vh;
    }
  }
</style>
