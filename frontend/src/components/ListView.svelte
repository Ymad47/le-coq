<script lang="ts">
  import type { Entity } from '../lib/types';
  import { entityIso2, countryLabel } from '../lib/countries';
  import { createEventDispatcher } from 'svelte';
  import Icon from './Icon.svelte';

  export let entities: Entity[] = [];
  export let loading = false;
  /** Current search text, used to word the empty state. */
  export let query = '';
  /** True when the type filter hides some results. */
  export let filtered = false;

  const SUGGESTIONS = ['TotalEnergies', 'Union européenne', 'Sahel', 'OTAN', 'Taiwan'];

  const dispatch = createEventDispatcher<{
    openArticle: Entity;
    suggest: { query: string };
    clearFilters: void;
  }>();

  let groupBy: 'country' | 'type' = 'country';

  const NO_GROUP = '—';

  function groupKey(e: Entity, by: 'country' | 'type'): string {
    if (by === 'type') return e.entity_type ?? e.type ?? NO_GROUP;
    // Normalized ISO-2 so 'FR' (gleif) and 'France' (eurostat) share a group
    return entityIso2(e) ?? e.country ?? e.properties?.country ?? NO_GROUP;
  }

  function buildGroups(list: Entity[], by: 'country' | 'type'): [string, Entity[]][] {
    const map = new Map<string, Entity[]>();
    for (const e of list) {
      const k = groupKey(e, by);
      if (!map.has(k)) map.set(k, []);
      map.get(k)!.push(e);
    }
    // Biggest groups first, ungrouped last
    return [...map.entries()].sort((a, b) => {
      if (a[0] === NO_GROUP) return 1;
      if (b[0] === NO_GROUP) return -1;
      return b[1].length - a[1].length || a[0].localeCompare(b[0]);
    });
  }

  $: groups = buildGroups(entities, groupBy);

  function groupLabel(k: string): string {
    if (k === NO_GROUP) return groupBy === 'type' ? 'Type inconnu' : 'Sans pays';
    if (groupBy === 'country' && /^[A-Z]{2}$/.test(k)) return countryLabel(k);
    return k;
  }
</script>

<section class="results" aria-busy={loading}>
  {#if loading}
    <div class="grid" aria-label="Chargement des résultats">
      {#each Array(6) as _}
        <div class="skeleton card-skeleton"></div>
      {/each}
    </div>
  {:else if entities.length === 0}
    <div class="empty panel">
      {#if filtered}
        <h2>Aucune entité pour ces filtres</h2>
        <p>Des résultats existent mais sont masqués par le filtre de type.</p>
        <button class="secondary" on:click={() => dispatch('clearFilters')}>Réinitialiser les filtres</button>
      {:else if query.trim()}
        <h2>Aucun résultat pour « {query} »</h2>
        <p>Rien en base pour l'instant. Interrogez les sources externes pour ingérer des données.</p>
        <button on:click={() => dispatch('suggest', { query })}>
          <span class="btn-inner"><Icon name="globe" size={16} /> Chercher « {query} » en ligne</span>
        </button>
      {:else}
        <h2>La base est vide</h2>
        <p>
          Lancez une recherche avec les sources externes activées : les résultats (Wikipedia, Wikidata,
          GLEIF…) sont ingérés et conservés en base.
        </p>
        <div class="suggestions">
          {#each SUGGESTIONS as s}
            <button class="chip" on:click={() => dispatch('suggest', { query: s })}>{s}</button>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <div class="view-toolbar">
      <span class="stats">
        <strong class="num">{entities.length}</strong> entité{entities.length !== 1 ? 's' : ''} ·
        <span class="num">{groups.length}</span> groupe{groups.length !== 1 ? 's' : ''}
      </span>
      <label class="group-by">
        Grouper par
        <select bind:value={groupBy}>
          <option value="country">pays</option>
          <option value="type">type</option>
        </select>
      </label>
    </div>

    {#each groups as [key, list] (key)}
      <section class="group">
        <h2 class="group-title">
          {groupLabel(key)}
          {#if groupBy === 'country' && /^[A-Z]{2}$/.test(key)}<span class="mono code">{key}</span>{/if}
          <span class="group-count num">{list.length}</span>
        </h2>
        <div class="grid">
          {#each list as e (e.id)}
            {@const desc = e.description ?? e.properties?.description}
            {@const src = e.properties?.source}
            {@const iso = entityIso2(e)}
            <button type="button" class="card" on:click={() => dispatch('openArticle', e)}>
              <span class="card-head">
                <span class="badge">{e.entity_type ?? e.type ?? 'entité'}</span>
                {#if src}<span class="src-badge">{src}</span>{/if}
              </span>
              <span class="card-title">{e.name}</span>
              {#if desc}<span class="card-desc">{desc}</span>{/if}
              <span class="card-foot">
                <span class="mono card-id" title={e.id}>{e.id}</span>
                {#if iso}<span class="mono">{iso}</span>{/if}
              </span>
            </button>
          {/each}
        </div>
      </section>
    {/each}
  {/if}
</section>

<style>
  .group {
    margin-bottom: 1.75rem;
  }

  .group-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0 0 0.6rem;
    font-size: var(--fs-md);
    font-weight: 650;
  }

  .code {
    color: var(--text-3);
    font-weight: 500;
  }

  .group-count {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-2);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 0.45rem;
  }

  .group-by {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--text-2);
    font-size: var(--fs-xs);
  }

  .group-by select {
    padding: 0.25rem 0.5rem;
    font-size: var(--fs-xs);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 0.75rem;
  }

  /* Card is a real <button>: reset the global button look */
  .card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.45rem;
    text-align: left;
    white-space: normal;
    font-weight: 400;
    font-size: var(--fs-base);
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    padding: 0.9rem 1rem 0.75rem;
    transition: border-color 0.15s var(--ease), box-shadow 0.15s var(--ease);
  }

  .card:hover:not(:disabled) {
    background: var(--surface);
    border-color: var(--brand-line);
    box-shadow: var(--shadow);
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }

  .card-title {
    font-size: var(--fs-md);
    font-weight: 600;
    line-height: 1.3;
  }

  .card-desc {
    font-size: var(--fs-sm);
    color: var(--text-2);
    line-height: 1.5;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-foot {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    margin-top: auto;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border);
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .card-id {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-skeleton {
    height: 140px;
  }

  .empty {
    text-align: center;
    padding: 3rem 1.5rem;
    max-width: 620px;
    margin: 2rem auto;
  }

  .empty h2 {
    margin: 0 0 0.5rem;
    font-size: var(--fs-lg);
  }

  .empty p {
    color: var(--text-2);
    margin: 0 0 1.25rem;
  }

  .btn-inner {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
  }

  .suggestions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.5rem;
  }
</style>
