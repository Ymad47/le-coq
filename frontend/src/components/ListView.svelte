<script lang="ts">
  import type { Entity } from '../lib/types';
  import { typeBadge } from '../lib/utils';
  import { createEventDispatcher } from 'svelte';

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
    return e.country ?? e.properties?.country ?? NO_GROUP;
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
    return k;
  }
</script>

<section class="results">
  {#if loading}
    <div class="grid">
      {#each Array(6) as _}
        <div class="skeleton card-skeleton"></div>
      {/each}
    </div>
  {:else if entities.length === 0}
    <div class="empty panel">
      {#if filtered}
        <h2>Aucune entité pour ces filtres</h2>
        <p>Les résultats existent mais sont masqués par le filtre de type.</p>
        <button class="secondary" on:click={() => dispatch('clearFilters')}>Réinitialiser les filtres</button>
      {:else if query.trim()}
        <h2>Aucun résultat pour « {query} »</h2>
        <p>Rien en base pour l'instant. Lancez une recherche sur les sources externes pour l'ingérer.</p>
        <button on:click={() => dispatch('suggest', { query })}>🌐 Chercher « {query} » en ligne</button>
      {:else}
        <h2>La base est vide</h2>
        <p>
          Lancez une recherche avec <strong>Sources externes</strong> activé : les résultats
          (Wikipedia, Wikidata, GLEIF…) sont ingérés et mis en cache.
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
        {entities.length} entité{entities.length !== 1 ? 's' : ''} · {groups.length}
        groupe{groups.length !== 1 ? 's' : ''}
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
      <div class="group">
        <h2 class="group-title">
          {groupBy === 'type' ? typeBadge(key === NO_GROUP ? null : key) : '🌍'}
          {groupLabel(key)} <span class="group-count">{list.length}</span>
        </h2>
        <div class="grid">
          {#each list as e (e.id)}
            {@const desc = e.description ?? e.properties?.description}
            {@const src = e.properties?.source}
            <button type="button" class="card" on:click={() => dispatch('openArticle', e)}>
              <span class="card-head">
                <span class="badge">{typeBadge(e.entity_type ?? e.type)} {e.entity_type ?? e.type ?? 'entité'}</span>
                {#if src}<span class="src-badge">{src}</span>{/if}
              </span>
              <span class="card-title">{e.name}</span>
              {#if desc}<span class="card-desc">{desc}</span>{/if}
            </button>
          {/each}
        </div>
      </div>
    {/each}
  {/if}
</section>

<style>
  .group {
    margin-bottom: 1.75rem;
  }

  .group-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--teal);
    margin: 0 0 0.6rem;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .group-count {
    color: var(--text-dimmer);
    font-weight: 500;
    font-size: 0.85em;
    background: var(--surface-strong);
    border-radius: 999px;
    padding: 0 0.5rem;
  }

  .group-by {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--text-dim);
    font-size: 0.82rem;
  }

  .group-by select {
    padding: 0.25rem 0.5rem;
    font-size: 0.82rem;
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
    gap: 0.4rem;
    text-align: left;
    white-space: normal;
    font-weight: 400;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 0.9rem 1.1rem;
    transition: background 0.15s, border-color 0.15s, transform 0.15s;
  }

  .card:hover:not(:disabled) {
    background: var(--surface-hover);
    border-color: rgba(45, 212, 191, 0.35);
    transform: translateY(-1px);
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }

  .card-title {
    font-size: 0.98rem;
    font-weight: 600;
  }

  .card-desc {
    font-size: 0.84rem;
    color: var(--text-dim);
    line-height: 1.45;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-skeleton {
    height: 120px;
  }

  .empty {
    text-align: center;
    padding: 3rem 1.5rem;
    max-width: 640px;
    margin: 2rem auto;
  }

  .empty h2 {
    margin: 0 0 0.5rem;
    font-size: 1.15rem;
  }

  .empty p {
    color: var(--text-dim);
    margin: 0 0 1.25rem;
  }

  .suggestions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.5rem;
  }

  @media (prefers-reduced-motion: reduce) {
    .card {
      transition: none;
    }
    .card:hover:not(:disabled) {
      transform: none;
    }
  }
</style>
