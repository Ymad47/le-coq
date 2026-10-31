<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import type { View } from '../lib/types';
  import * as api from '../lib/api';
  import { entityIso2 } from '../lib/countries';
  import Logo from './Logo.svelte';
  import Icon, { type IconName } from './Icon.svelte';
  import Globe from './Globe.svelte';

  export let query = '';
  export let ingestOnline = true;
  export let sources: string[] = [];
  export let healthOk: boolean | null = null;

  const dispatch = createEventDispatcher<{
    search: { query: string };
    navigate: { view: View };
  }>();

  const SUGGESTIONS = ['TotalEnergies', 'Union européenne', 'Sahel', 'OTAN', 'Taiwan'];

  const VIEWS: { id: View; key: string; icon: IconName; title: string; text: string }[] = [
    {
      id: 'list',
      key: '1',
      icon: 'list',
      title: 'Liste',
      text: 'Les résultats groupés par pays ou par type, avec leur source et leur identifiant.',
    },
    {
      id: 'graph',
      key: '2',
      icon: 'graph',
      title: 'Graphe',
      text: 'Les résultats et leurs voisins directs : qui est lié à qui, et par quelle relation.',
    },
    {
      id: 'focus',
      key: '3',
      icon: 'focus',
      title: 'Focus',
      text: 'Une entité au centre et son réseau sur deux niveaux, pour remonter une chaîne de liens.',
    },
    {
      id: 'map',
      key: '4',
      icon: 'map',
      title: 'Carte',
      text: "La densité d'entités par pays sur toute la base, pour voir où l'information se concentre.",
    },
  ];

  const PRINCIPLES = [
    {
      title: 'Souverain',
      text: "Auto-hébergé : la base, l'API et les plugins tournent sur votre infrastructure. Les sources externes ne sont interrogées que si l'option est activée.",
    },
    {
      title: 'Traçable',
      text: 'Chaque entité conserve sa source ; chaque relation, son type et son niveau de confiance.',
    },
    {
      title: 'Ouvert',
      text: 'Code sous licence AGPL-3.0, plugins Python lisibles et extensibles, données issues de sources ouvertes.',
    },
  ];

  const SAMPLE = 1000;
  let entityCount: number | null = null;
  let entityCapped = false;
  let countryCount: number | null = null;
  let statsError = false;

  onMount(async () => {
    try {
      const list = await api.listEntities(SAMPLE);
      entityCount = list.length;
      entityCapped = list.length >= SAMPLE;
      countryCount = new Set(list.map((e) => entityIso2(e)).filter(Boolean)).size;
    } catch {
      statsError = true;
    }
  });

  function fmt(n: number | null, capped = false): string {
    if (statsError || n === null) return '—';
    return `${n.toLocaleString('fr-FR')}${capped ? '+' : ''}`;
  }

  function submit() {
    if (query.trim()) dispatch('search', { query });
  }
</script>

<div class="home">
  <!-- ============ Hero ============ -->
  <section class="hero" aria-labelledby="hero-title">
    <div class="hero-copy">
      <Logo variant="full" size={52} />

      <h1 id="hero-title">
        <span lang="en">Understand the world. Connect the signals.</span>
        <span class="tagline-fr">Comprendre le monde. Relier les signaux.</span>
      </h1>

      <p class="lead">
        Le Coq rassemble des sources ouvertes (Wikipedia, Wikidata, GLEIF, Eurostat, GDELT…) dans une
        base unique et révèle les liens entre organisations, personnes, pays et événements.
      </p>

      <form class="hero-search" role="search" on:submit|preventDefault={submit}>
        <label class="sr-only" for="home-search">Rechercher une entité</label>
        <span class="search-icon"><Icon name="search" size={18} /></span>
        <input
          id="home-search"
          type="search"
          bind:value={query}
          placeholder="Une organisation, une personne, un pays…"
          autocomplete="off"
        />
        <button type="submit" disabled={!query.trim()}>Explorer</button>
      </form>

      <div class="search-options">
        <label class="switch">
          <input type="checkbox" bind:checked={ingestOnline} />
          Interroger aussi les sources externes
        </label>
        <div class="suggestions" aria-label="Exemples de recherche">
          {#each SUGGESTIONS as s}
            <button type="button" class="chip" on:click={() => dispatch('search', { query: s })}>{s}</button>
          {/each}
        </div>
      </div>
    </div>

    <div class="hero-visual">
      <Globe />
    </div>
  </section>

  <!-- ============ Live status (real data only) ============ -->
  <section class="metrics" aria-label="État de la plateforme">
    <div class="metric">
      <span class="metric-label">Entités en base</span>
      <span class="metric-value num">{fmt(entityCount, entityCapped)}</span>
    </div>
    <div class="metric">
      <span class="metric-label">Pays localisés</span>
      <span class="metric-value num">{fmt(countryCount)}</span>
      {#if entityCapped}<span class="metric-note">sur un échantillon de {SAMPLE.toLocaleString('fr-FR')}</span>{/if}
    </div>
    <div class="metric">
      <span class="metric-label">Sources connectées</span>
      <span class="metric-value num">{sources.length || '—'}</span>
      {#if sources.length}<span class="metric-note mono">{sources.slice(0, 4).join(' · ')}{sources.length > 4 ? ' …' : ''}</span>{/if}
    </div>
    <div class="metric">
      <span class="metric-label">API</span>
      <span class="status metric-status" class:ok={healthOk === true} class:down={healthOk === false}>
        {healthOk === null ? 'Vérification…' : healthOk ? 'Opérationnelle' : 'Injoignable'}
      </span>
    </div>
  </section>

  <!-- ============ Views ============ -->
  <section class="section" aria-labelledby="views-title">
    <header class="section-head">
      <span class="eyebrow">Explorer</span>
      <h2 id="views-title">Quatre façons de lire les données</h2>
    </header>
    <div class="views">
      {#each VIEWS as v}
        <button type="button" class="view-card" on:click={() => dispatch('navigate', { view: v.id })}>
          <span class="view-top">
            <span class="view-icon"><Icon name={v.icon} size={20} /></span>
            <kbd>{v.key}</kbd>
          </span>
          <span class="view-title">{v.title}</span>
          <span class="view-text">{v.text}</span>
          <span class="view-cta">Ouvrir <Icon name="arrowRight" size={14} /></span>
        </button>
      {/each}
    </div>
  </section>

  <!-- ============ Principles ============ -->
  <section class="section" aria-labelledby="principles-title">
    <header class="section-head">
      <span class="eyebrow">Principes</span>
      <h2 id="principles-title">Fabriqué en France, construit pour durer</h2>
    </header>
    <div class="principles">
      {#each PRINCIPLES as p, i}
        <article class="principle">
          <span class="principle-num mono">0{i + 1}</span>
          <h3>{p.title}</h3>
          <p>{p.text}</p>
        </article>
      {/each}
    </div>
  </section>

  <footer class="footer">
    <Logo variant="horizontal" tone="mono" size={24} label="" />
    <span>Geopolitical Intelligence Platform · Fabriqué en France · AGPL-3.0</span>
  </footer>
</div>

<style>
  .home {
    max-width: 1200px;
    margin: 0 auto;
    padding: 0 1.5rem 3rem;
  }

  /* ---------- Hero ---------- */

  .hero {
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr);
    align-items: center;
    gap: 3rem;
    /* Fill the first screen on laptops, but never stretch on tall monitors */
    min-height: min(calc(100vh - var(--topbar-h) - 2rem), 760px);
    padding: 2.5rem 0;
  }

  .hero-copy {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1.5rem;
  }

  h1 {
    margin: 0.5rem 0 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    font-size: clamp(2rem, 4.2vw, 3.25rem);
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1.08;
  }

  .tagline-fr {
    font-size: 0.42em;
    font-weight: 500;
    letter-spacing: -0.005em;
    line-height: 1.3;
    color: var(--text-2);
    padding-left: 0.85rem;
    border-left: 2px solid var(--brand-text);
  }

  .lead {
    margin: 0;
    max-width: 36rem;
    font-size: var(--fs-md);
    line-height: 1.65;
    color: var(--text-2);
  }

  .hero-search {
    position: relative;
    display: flex;
    gap: 0.5rem;
    width: 100%;
    max-width: 36rem;
    padding: 0.35rem;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow);
  }

  .hero-search:focus-within {
    border-color: var(--brand-text);
    box-shadow: 0 0 0 3px var(--brand-soft), var(--shadow);
  }

  .search-icon {
    position: absolute;
    left: 0.95rem;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-3);
    pointer-events: none;
  }

  .hero-search input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    font-size: var(--fs-md);
    padding: 0.6rem 0.75rem 0.6rem 2.4rem;
  }

  .hero-search input:focus {
    box-shadow: none;
  }

  .hero-search button {
    padding: 0.6rem 1.35rem;
    font-size: var(--fs-base);
  }

  .search-options {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .switch {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: var(--fs-sm);
    color: var(--text-2);
    cursor: pointer;
  }

  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .hero-visual {
    width: 100%;
    max-width: 520px;
    aspect-ratio: 1;
    justify-self: center;
  }

  /* ---------- Metrics ---------- */

  .metrics {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
  }

  .metric {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 1.1rem 1.35rem;
    min-width: 0;
  }

  .metric + .metric {
    border-left: 1px solid var(--border);
  }

  .metric-label {
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
  }

  .metric-value {
    font-size: var(--fs-2xl);
    font-weight: 650;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }

  .metric-status {
    font-size: var(--fs-md);
    min-height: 2rem;
  }

  .metric-note {
    font-size: var(--fs-xs);
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ---------- Sections ---------- */

  .section {
    margin-top: 4.5rem;
  }

  .section-head {
    margin-bottom: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .section-head .eyebrow {
    color: var(--brand-text);
  }

  h2 {
    margin: 0;
    font-size: clamp(1.35rem, 2.4vw, 1.75rem);
    font-weight: 650;
  }

  .views {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
  }

  /* Cards are real buttons: reset the global button look */
  .view-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.6rem;
    text-align: left;
    white-space: normal;
    font-weight: 400;
    font-size: var(--fs-base);
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    padding: 1.15rem;
    transition: border-color 0.15s var(--ease), box-shadow 0.15s var(--ease);
  }

  .view-card:hover:not(:disabled) {
    background: var(--surface);
    border-color: var(--brand-line);
    box-shadow: var(--shadow);
  }

  .view-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
  }

  .view-icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius);
    background: var(--brand-soft);
    color: var(--brand-text);
  }

  .view-title {
    font-size: var(--fs-md);
    font-weight: 650;
  }

  .view-text {
    font-size: var(--fs-sm);
    line-height: 1.55;
    color: var(--text-2);
  }

  .view-cta {
    margin-top: auto;
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--brand-text);
  }

  .principles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .principle {
    padding: 1.35rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-top: 2px solid var(--brand);
    border-radius: 0 0 var(--radius-lg) var(--radius-lg);
  }

  .principle-num {
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .principle h3 {
    margin: 0.5rem 0;
    font-size: var(--fs-lg);
    font-weight: 650;
  }

  .principle p {
    margin: 0;
    font-size: var(--fs-sm);
    line-height: 1.6;
    color: var(--text-2);
  }

  .footer {
    margin-top: 4.5rem;
    padding-top: 1.5rem;
    border-top: 1px solid var(--border);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .footer :global(.logo) {
    --fg: var(--text-2);
  }

  /* ---------- Responsive ---------- */

  @media (max-width: 1000px) {
    .hero {
      /* minmax(0, …): let the column shrink below its content's min width */
      grid-template-columns: minmax(0, 1fr);
      min-height: 0;
      gap: 1.5rem;
      padding-top: 1.5rem;
    }

    .hero-visual {
      order: -1;
      max-width: 320px;
    }

    .views {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .principles {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 640px) {
    .home {
      padding: 0 1rem 2rem;
    }

    .hero-visual {
      max-width: 240px;
    }

    .metrics {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .metric:nth-child(3) {
      border-left: none;
    }

    .metric:nth-child(n + 3) {
      border-top: 1px solid var(--border);
    }

    .views {
      grid-template-columns: minmax(0, 1fr);
    }

    .hero-search button {
      padding: 0.6rem 1rem;
    }
  }
</style>
