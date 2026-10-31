<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import type { Entity, ArticleRelation } from '../lib/types';
  import { typeBadge, stripSourcePrefix } from '../lib/utils';
  import { getEntitySummary, getNeighborhood } from '../lib/api';

  export let entity: Entity | null = null;

  const dispatch = createEventDispatcher<{
    close: void;
    openInFocus: { id: string; name: string };
  }>();

  let articleRelations: ArticleRelation[] = [];
  let articleSummary: string | null = null;
  let loadingData = false;

  $: if (entity) loadEntityData(entity);

  async function loadEntityData(e: Entity) {
    loadingData = true;
    articleRelations = [];
    articleSummary = null;
    try {
      // Only this entity's own relations, not the whole table
      const [hood, summaryRes] = await Promise.all([
        getNeighborhood([e.id], 500),
        getEntitySummary(e.id).catch(() => null),
      ]);
      const rels = hood.relationships;
      const id = e.id;
      articleRelations = rels
        .filter((r) => {
          const from = (r as any).source_id ?? r.from_entity;
          const to = (r as any).target_id ?? r.to_entity;
          return from === id || to === id;
        })
        .map((r) => {
          const from = (r as any).source_id ?? r.from_entity;
          const to = (r as any).target_id ?? r.to_entity;
          const relType = (r as any).relation_type ?? r.relationship_type ?? (r as any).type ?? '';
          return { from, to, relType, outgoing: from === id };
        });
      articleSummary = summaryRes?.summary ?? null;
    } catch {
      articleRelations = [];
      articleSummary = null;
    } finally {
      loadingData = false;
    }
  }

  function close() {
    dispatch('close');
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && entity) close();
  }

  function handleOpenInFocus() {
    if (entity) {
      dispatch('openInFocus', { id: entity.id, name: entity.name });
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleGlobalKeydown);
    return () => window.removeEventListener('keydown', handleGlobalKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleGlobalKeydown);
  });
</script>

{#if entity}
  <div
    class="drawer-overlay"
    on:click={close}
    on:keydown={(e) => e.key === 'Escape' && close()}
    role="presentation"
    tabindex="-1"
  ></div>
  <aside class="article-drawer" role="dialog" aria-modal="true" aria-label="Détail de l'entité" data-id={entity.id}>
    <button class="close-drawer" type="button" on:click={(e) => { e.stopPropagation(); close(); }} aria-label="Fermer">
      ×
    </button>

    <div class="drawer-head">
      <span class="badge">
        {typeBadge(entity.entity_type ?? entity.type)} {entity.entity_type ?? entity.type ?? 'entité'}
      </span>
    </div>
    <h2 class="drawer-title">{entity.name}</h2>

    {#if loadingData}
      <p class="loading">Chargement des relations…</p>
    {:else}
      {#if articleSummary}
        <section class="drawer-summary">
          <h3 class="section-title">📖 Résumé</h3>
          <p class="summary-text">{articleSummary}</p>
        </section>
      {/if}

      {#if entity.description ?? entity.properties?.description}
        <section class="drawer-section">
          <h3 class="section-title">Description</h3>
          <p class="drawer-desc">{entity.description ?? entity.properties?.description}</p>
        </section>
      {/if}

      {#if entity.country ?? entity.properties?.country}
        <div class="drawer-kv"><span class="k">Pays</span><span class="v">{entity.country ?? entity.properties?.country}</span></div>
      {/if}
      {#if entity.properties?.source}
        <div class="drawer-kv"><span class="k">Source</span><span class="v src-badge">{entity.properties.source}</span></div>
      {/if}

      {#if entity.properties && Object.keys(entity.properties).filter((k) => !['description', 'country', 'source'].includes(k)).length > 0}
        <section class="drawer-section">
          <h3 class="section-title">Propriétés</h3>
          {#each Object.entries(entity.properties).filter(([k]) => !['description', 'country', 'source'].includes(k)) as [k, v]}
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
                  <span class="rel-dir">→</span> <strong>{r.relType}</strong>
                  <span class="rel-target">{stripSourcePrefix(r.to)}</span>
                {:else}
                  <span class="rel-dir">←</span> <strong>{r.relType}</strong>
                  <span class="rel-target">{stripSourcePrefix(r.from)}</span>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      <div class="drawer-actions">
        <button class="secondary small" on:click={handleOpenInFocus}>
          🎯 Ouvrir en Vue Focus
        </button>
      </div>
    {/if}
  </aside>
{/if}

<style>
  .drawer-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    z-index: 99;
  }

  .article-drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: 480px;
    background: rgba(10, 10, 15, 0.95);
    backdrop-filter: blur(40px) saturate(180%);
    border-left: 1px solid rgba(255, 255, 255, 0.1);
    padding: 1.5rem;
    overflow-y: auto;
    z-index: 100;
    box-shadow: -10px 0 40px rgba(0, 0, 0, 0.6);
  }

  .close-drawer {
    position: fixed;
    top: 1rem;
    right: 1rem;
    width: 36px;
    height: 36px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 50%;
    color: #e2e8f0;
    font-size: 1.3rem;
    cursor: pointer;
    z-index: 200;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close-drawer:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .drawer-head {
    margin-bottom: 0.75rem;
  }

  .drawer-title {
    font-size: 1.4rem;
    margin: 0 0 1rem 0;
    color: #e2e8f0;
  }

  .drawer-summary {
    background: rgba(45, 212, 191, 0.08);
    border-left: 3px solid #2dd4bf;
    padding: 0.85rem 1rem;
    border-radius: 0 12px 12px 0;
    margin-bottom: 1rem;
  }

  .summary-text {
    color: #cbd5e1;
    font-size: 0.9rem;
    line-height: 1.6;
    margin: 0;
  }

  .drawer-section {
    margin-bottom: 1.25rem;
  }

  .section-title {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #64748b;
    margin: 0 0 0.5rem 0;
    font-weight: 500;
  }

  .drawer-desc {
    color: #94a3b8;
    font-size: 0.9rem;
    line-height: 1.6;
    margin: 0;
  }

  .drawer-kv {
    display: flex;
    justify-content: space-between;
    padding: 0.4rem 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.85rem;
  }

  .drawer-kv .k {
    color: #94a3b8;
  }

  .drawer-kv .v {
    color: #e2e8f0;
    text-align: right;
    max-width: 60%;
    word-break: break-word;
  }

  .rel-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .rel-item {
    padding: 0.4rem 0;
    font-size: 0.85rem;
    color: #cbd5e1;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .rel-dir {
    color: #2dd4bf;
    margin-right: 0.3rem;
  }

  .rel-target {
    color: #94a3b8;
  }

  .drawer-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 1.5rem;
    padding-top: 1rem;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .loading {
    color: #94a3b8;
    font-style: italic;
    text-align: center;
    padding: 2rem 0;
  }
</style>
