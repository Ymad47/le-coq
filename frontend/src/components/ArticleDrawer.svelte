<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import type { Entity, ArticleRelation } from '../lib/types';
  import { stripSourcePrefix } from '../lib/utils';
  import Icon from './Icon.svelte';
  import { getEntitySummary, getNeighborhood } from '../lib/api';

  export let entity: Entity | null = null;

  const dispatch = createEventDispatcher<{
    close: void;
    openInFocus: { id: string; name: string };
  }>();

  let articleRelations: ArticleRelation[] = [];
  let articleSummary: string | null = null;
  let relationsTruncated = false;
  let loadingData = false;

  const MAX_RELATIONS = 500;

  // Opening B while A is still loading must not show A's data under B's title
  let requestToken = 0;

  $: if (entity) loadEntityData(entity);

  async function loadEntityData(e: Entity) {
    const token = ++requestToken;
    loadingData = true;
    articleRelations = [];
    articleSummary = null;
    relationsTruncated = false;
    try {
      // Only this entity's own relations (the endpoint returns nothing else)
      const [hood, summaryRes] = await Promise.all([
        getNeighborhood([e.id], MAX_RELATIONS),
        getEntitySummary(e.id).catch(() => null),
      ]);
      if (token !== requestToken) return;
      articleRelations = hood.relationships.map((r) => ({
        from: r.from_entity,
        to: r.to_entity,
        relType: r.relationship_type ?? '',
        outgoing: r.from_entity === e.id,
      }));
      relationsTruncated = hood.truncated;
      articleSummary = summaryRes?.summary ?? null;
    } catch {
      if (token !== requestToken) return;
      articleRelations = [];
      articleSummary = null;
    } finally {
      if (token === requestToken) loadingData = false;
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
    <button class="ghost icon close-drawer" type="button" on:click={(e) => { e.stopPropagation(); close(); }} aria-label="Fermer">
      <Icon name="close" size={18} />
    </button>

    <div class="drawer-head">
      <span class="badge brand">{entity.entity_type ?? entity.type ?? 'entité'}</span>
    </div>
    <h2 class="drawer-title">{entity.name}</h2>
    <p class="drawer-id mono" title="Identifiant">{entity.id}</p>

    {#if loadingData}
      <p class="loading">Chargement des relations…</p>
    {:else}
      {#if articleSummary}
        <section class="drawer-summary">
          <h3 class="section-title">Résumé</h3>
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
        <div class="drawer-kv"><span class="k">Source</span><span class="v mono">{entity.properties.source}</span></div>
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
          <h3 class="section-title">
            Relations ({articleRelations.length}{relationsTruncated ? '+' : ''})
          </h3>
          {#if relationsTruncated}
            <p class="trunc-note">Les {MAX_RELATIONS} relations les plus fiables sont affichées.</p>
          {/if}
          <ul class="rel-list">
            {#each articleRelations as r (r.from + '|' + r.relType + '|' + r.to)}
              <li class="rel-item">
                {#if r.outgoing}
                  <span class="rel-dir" aria-label="sortante"><Icon name="arrowRight" size={14} /></span>
                  <span class="rel-type mono">{r.relType}</span>
                  <span class="rel-target">{stripSourcePrefix(r.to)}</span>
                {:else}
                  <span class="rel-dir" aria-label="entrante"><Icon name="arrowLeft" size={14} /></span>
                  <span class="rel-type mono">{r.relType}</span>
                  <span class="rel-target">{stripSourcePrefix(r.from)}</span>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      <div class="drawer-actions">
        <button on:click={handleOpenInFocus}>
          <Icon name="focus" size={16} /> Ouvrir en vue Focus
        </button>
      </div>
    {/if}
  </aside>
{/if}

<style>
  .drawer-overlay {
    position: fixed;
    inset: 0;
    background: var(--scrim);
    z-index: 99;
  }

  .article-drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(480px, 100vw);
    background: var(--surface);
    border-left: 1px solid var(--border);
    padding: 1.5rem;
    overflow-y: auto;
    z-index: 100;
    box-shadow: var(--shadow);
  }

  .close-drawer {
    position: absolute;
    top: 1rem;
    right: 1rem;
  }

  .drawer-head {
    margin-bottom: 0.6rem;
  }

  .drawer-title {
    font-size: var(--fs-xl);
    font-weight: 700;
    margin: 0 2.5rem 0.25rem 0;
  }

  .drawer-id {
    margin: 0 0 1.25rem;
    color: var(--text-3);
    font-size: var(--fs-xs);
    overflow-wrap: anywhere;
  }

  .drawer-summary {
    background: var(--brand-soft);
    border-left: 3px solid var(--brand-text);
    padding: 0.85rem 1rem;
    border-radius: 0 var(--radius) var(--radius) 0;
    margin-bottom: 1.25rem;
  }

  .summary-text {
    font-size: var(--fs-sm);
    line-height: 1.6;
    margin: 0;
  }

  .drawer-section {
    margin-bottom: 1.25rem;
  }

  .section-title {
    font-size: var(--fs-xs);
    text-transform: uppercase;
    letter-spacing: var(--tracking-caps);
    color: var(--text-3);
    margin: 0 0 0.5rem;
    font-weight: 600;
  }

  .drawer-desc {
    color: var(--text-2);
    font-size: var(--fs-sm);
    line-height: 1.6;
    margin: 0;
  }

  .drawer-kv {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.4rem 0;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-sm);
  }

  .drawer-kv .k {
    color: var(--text-2);
  }

  .drawer-kv .v {
    text-align: right;
    max-width: 60%;
    overflow-wrap: anywhere;
  }

  .rel-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .rel-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0;
    font-size: var(--fs-sm);
    border-bottom: 1px solid var(--border);
  }

  .rel-dir {
    color: var(--brand-text);
    display: inline-flex;
  }

  .rel-type {
    color: var(--text-2);
    font-size: var(--fs-xs);
  }

  .rel-target {
    margin-left: auto;
    text-align: right;
  }

  .drawer-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 1.5rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
  }

  .drawer-actions button {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
  }

  .loading {
    color: var(--text-2);
    text-align: center;
    padding: 2rem 0;
  }

  .trunc-note {
    font-size: var(--fs-xs);
    color: var(--text-3);
    margin: -0.25rem 0 0.5rem;
  }
</style>
