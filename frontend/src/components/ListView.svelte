<script lang="ts">
  import type { Entity } from '../lib/types';
  import { typeBadge } from '../lib/utils';
  import { createEventDispatcher } from 'svelte';

  export let entities: Entity[] = [];
  export let loading: boolean = false;

  const dispatch = createEventDispatcher<{
    openArticle: Entity;
  }>();

  // Group entities by country, alphabetical
  $: groupedByCountry = (() => {
    const groups = new Map<string, Entity[]>();
    for (const e of entities) {
      const country = e.country ?? e.properties?.country ?? 'N/A';
      if (!groups.has(country)) groups.set(country, []);
      groups.get(country)!.push(e);
    }
    return Array.from(groups.entries()).sort((a, b) => a[0].localeCompare(b[0]));
  })();

  function handleOpenArticle(e: Entity) {
    dispatch('openArticle', e);
  }
</script>

<section class="results">
  <p class="count">
    {entities.length} entité{entities.length !== 1 ? 's' : ''} ·
    {groupedByCountry.length} pays
  </p>

  {#each groupedByCountry as [country, list] (country)}
    <div class="country-group">
      <h2 class="country-title">
        🌍 {country} <span class="country-count">({list.length})</span>
      </h2>
      <div class="country-grid">
        {#each list as e (e.id)}
          {@const desc = e.description ?? e.properties?.description}
          {@const src = e.properties?.source}
          <article
            class="card clickable"
            on:click={() => handleOpenArticle(e)}
            role="button"
            tabindex="0"
            on:keydown={(ev) => ev.key === 'Enter' && handleOpenArticle(e)}
          >
            <div class="card-head">
              <span class="badge">
                {typeBadge(e.entity_type ?? e.type)} {e.entity_type ?? e.type ?? 'entité'}
              </span>
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
    {#if !loading}
      <p class="empty">Aucune entité. Lancez une ingestion via le CLI.</p>
    {/if}
  {/each}
</section>

<style>
  /* List view specific styles — inherited from design tokens in App.svelte */
  .results {
    padding: 1rem 0;
  }

  .count {
    color: #94a3b8;
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }

  .country-group {
    margin-bottom: 2rem;
  }

  .country-title {
    font-size: 1.1rem;
    color: #2dd4bf;
    margin-bottom: 0.75rem;
    font-weight: 500;
  }

  .country-count {
    color: #64748b;
    font-weight: 400;
    font-size: 0.9em;
  }

  .country-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 1rem;
  }

  .card {
    background: rgba(255, 255, 255, 0.03);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    padding: 1rem 1.25rem;
    transition: all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94);
  }

  .card.clickable {
    cursor: pointer;
  }

  .card.clickable:hover {
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(45, 212, 191, 0.3);
    transform: translateY(-1px);
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .badge {
    background: rgba(45, 212, 191, 0.12);
    color: #2dd4bf;
    padding: 0.2rem 0.6rem;
    border-radius: 999px;
    font-size: 0.7rem;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .src-badge {
    background: rgba(148, 163, 184, 0.15);
    color: #94a3b8;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    font-size: 0.65rem;
  }

  .card h3 {
    font-size: 1rem;
    margin: 0.5rem 0;
    color: #e2e8f0;
    font-weight: 600;
  }

  .card p {
    font-size: 0.85rem;
    color: #94a3b8;
    line-height: 1.5;
    margin: 0.5rem 0;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-action {
    color: #2dd4bf;
    font-size: 0.8rem;
    margin-top: 0.5rem;
    display: inline-block;
  }

  .empty {
    text-align: center;
    color: #64748b;
    padding: 3rem 0;
    font-style: italic;
  }
</style>
