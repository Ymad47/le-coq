<script lang="ts">
  import { onMount } from 'svelte';
  import type { TimelineArticle, TimelineResponse } from '../lib/types';
  import * as api from '../lib/api';
  import Icon from './Icon.svelte';

  /** Search text from the top bar. */
  export let query = '';

  const DAY_MS = 86_400_000;
  const isoDay = (d: Date) => d.toISOString().slice(0, 10);
  const today = new Date();

  // Default period: the last 30 days
  let from = isoDay(new Date(today.getTime() - 30 * DAY_MS));
  let to = isoDay(today);

  let loading = false;
  let fetchingOnline = false;
  let error = '';
  let data: TimelineResponse | null = null;

  /** Outlets / countries hidden by the user (empty = show all). */
  let hiddenDomains = new Set<string>();
  let hiddenCountries = new Set<string>();

  // Each load gets a token; a slower, outdated response is dropped.
  let requestToken = 0;

  /**
   * Loads the timeline for `query`. With online=true the backend first asks
   * GDELT for the period (external request, slow), otherwise local data only.
   */
  export async function load(opts: { online?: boolean } = {}): Promise<void> {
    if (!query.trim()) {
      data = null;
      error = '';
      return;
    }
    if (from && to && from > to) {
      error = 'La date de début doit précéder la date de fin.';
      return;
    }
    const token = ++requestToken;
    loading = true;
    fetchingOnline = !!opts.online;
    error = '';
    try {
      const res = await api.getTimeline(query, { from, to, online: opts.online, limit: 2000 });
      if (token !== requestToken) return;
      data = res;
      hiddenDomains = new Set();
      hiddenCountries = new Set();
    } catch (e: any) {
      if (token === requestToken) error = `Chronologie indisponible : ${e.message}`;
    } finally {
      if (token === requestToken) {
        loading = false;
        fetchingOnline = false;
      }
    }
  }

  function count<T>(items: T[], key: (t: T) => string): [string, number][] {
    const m = new Map<string, number>();
    for (const it of items) {
      const k = key(it) || '—';
      m.set(k, (m.get(k) ?? 0) + 1);
    }
    return [...m.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  }

  $: all = data?.articles ?? [];
  $: domains = count(all, (a) => a.domain);
  $: countries = count(all, (a) => a.source_country);
  $: visible = all.filter(
    (a) => !hiddenDomains.has(a.domain || '—') && !hiddenCountries.has(a.source_country || '—')
  );

  // Group by UTC day, oldest first (the API already sorts chronologically)
  $: byDay = (() => {
    const m = new Map<string, TimelineArticle[]>();
    for (const a of visible) {
      const d = a.published_at.slice(0, 10);
      if (!m.has(d)) m.set(d, []);
      m.get(d)!.push(a);
    }
    return [...m.entries()];
  })();

  // Volume per day over the whole selected period, including empty days
  $: volume = (() => {
    if (!data || !from || !to) return [] as { day: string; n: number }[];
    const counts = new Map(byDay.map(([d, list]) => [d, list.length]));
    const out: { day: string; n: number }[] = [];
    for (let t = Date.parse(`${from}T00:00:00Z`); t <= Date.parse(`${to}T00:00:00Z`); t += DAY_MS) {
      const day = new Date(t).toISOString().slice(0, 10);
      out.push({ day, n: counts.get(day) ?? 0 });
      if (out.length > 731) break; // safety: 2 years of bars at most
    }
    return out;
  })();
  $: maxVolume = Math.max(1, ...volume.map((v) => v.n));
  $: peak = volume.reduce((best, v) => (v.n > best.n ? v : best), { day: '', n: 0 });

  const dayFmt = new Intl.DateTimeFormat('fr-FR', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
    timeZone: 'UTC',
  });
  const shortFmt = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'short', timeZone: 'UTC' });
  const dayLabel = (d: string) => dayFmt.format(new Date(`${d}T00:00:00Z`));
  const shortLabel = (d: string) => shortFmt.format(new Date(`${d}T00:00:00Z`));
  const timeLabel = (iso: string) => iso.slice(11, 16);

  function toggle(set: Set<string>, key: string): Set<string> {
    if (set.has(key)) set.delete(key);
    else set.add(key);
    return new Set(set);
  }

  function jumpTo(day: string) {
    document.getElementById(`day-${day}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  // Local data only when the view opens (no external call). Later searches
  // are triggered by the parent through load().
  onMount(() => {
    if (query.trim()) load();
  });
</script>

<section class="timeline" aria-busy={loading}>
  <div class="view-toolbar controls">
    <label class="field">
      <span>Du</span>
      <input type="date" bind:value={from} max={to || undefined} />
    </label>
    <label class="field">
      <span>au</span>
      <input type="date" bind:value={to} min={from || undefined} />
    </label>
    <button class="secondary small" on:click={() => load()} disabled={loading || !query.trim()}>
      Afficher
    </button>
    <button
      class="small"
      on:click={() => load({ online: true })}
      disabled={loading || !query.trim()}
      title="Récupère les articles de la période auprès de GDELT, puis les conserve en base"
    >
      <span class="btn-inner"><Icon name="globe" size={14} /> Interroger GDELT</span>
    </button>
  </div>

  {#if !query.trim()}
    <div class="empty panel">
      <h2>Chronologie d'un événement</h2>
      <p>
        Saisissez un sujet dans la recherche (par exemple un lieu et un mot-clé), choisissez la période,
        puis interrogez GDELT : Le Coq affiche qui en a parlé, et quand.
      </p>
    </div>
  {:else}
    {#if error}
      <div class="alert error"><Icon name="alert" size={16} /><span>{error}</span></div>
    {/if}
    {#if fetchingOnline}
      <div class="alert info">
        <Icon name="info" size={16} />
        <span>Interrogation de GDELT pour la période… cela peut prendre jusqu'à une minute.</span>
      </div>
    {/if}
    {#if data?.errors?.length}
      <div class="alert warn"><Icon name="alert" size={16} /><span>{data.errors.join(' · ')}</span></div>
    {/if}

    {#if loading && !data}
      <div class="skeleton bars-skeleton"></div>
    {:else if data && all.length === 0}
      <div class="empty panel">
        <h2>Aucun article daté pour « {data.query} »</h2>
        <p>
          {#if data.ingested > 0}
            GDELT a renvoyé des résultats, mais aucun ne correspond à cette période.
          {:else}
            Rien en base pour cette période. Interrogez GDELT pour récupérer les articles, ou élargissez les dates.
          {/if}
        </p>
        <button on:click={() => load({ online: true })} disabled={loading}>
          <span class="btn-inner"><Icon name="globe" size={16} /> Interroger GDELT</span>
        </button>
      </div>
    {:else if data}
      <!-- Summary -->
      <div class="summary">
        <span><strong class="num">{visible.length}</strong> article{visible.length > 1 ? 's' : ''}</span>
        <span><strong class="num">{domains.length}</strong> média{domains.length > 1 ? 's' : ''}</span>
        <span><strong class="num">{countries.length}</strong> pays</span>
        {#if peak.n > 0}<span>pic le <strong>{shortLabel(peak.day)}</strong> ({peak.n})</span>{/if}
        {#if data.truncated}<span class="warn-text">limité aux 2 000 premiers</span>{/if}
      </div>

      <!-- Coverage volume per day -->
      <div class="volume panel" role="group" aria-label="Nombre d'articles par jour">
        <div class="bars">
          {#each volume as v (v.day)}
            <button
              class="bar"
              class:empty-bar={v.n === 0}
              style="--h: {Math.max(v.n ? 6 : 2, (v.n / maxVolume) * 100)}%"
              disabled={v.n === 0}
              title="{shortLabel(v.day)} : {v.n} article{v.n > 1 ? 's' : ''}"
              aria-label="{dayLabel(v.day)} : {v.n} article{v.n > 1 ? 's' : ''}"
              on:click={() => jumpTo(v.day)}
            ></button>
          {/each}
        </div>
        <div class="axis mono">
          <span>{volume.length ? shortLabel(volume[0].day) : ''}</span>
          <span>{volume.length ? shortLabel(volume[volume.length - 1].day) : ''}</span>
        </div>
      </div>

      <!-- Filters -->
      <div class="filters">
        <div class="filter-row">
          <span class="eyebrow">Médias</span>
          <div class="chips">
            {#each domains.slice(0, 12) as [d, n]}
              <button
                class="chip"
                class:active={!hiddenDomains.has(d)}
                aria-pressed={!hiddenDomains.has(d)}
                on:click={() => (hiddenDomains = toggle(hiddenDomains, d))}
              >
                <span class="mono">{d}</span> <span class="num">{n}</span>
              </button>
            {/each}
          </div>
        </div>
        <div class="filter-row">
          <span class="eyebrow">Pays du média</span>
          <div class="chips">
            {#each countries.slice(0, 12) as [c, n]}
              <button
                class="chip"
                class:active={!hiddenCountries.has(c)}
                aria-pressed={!hiddenCountries.has(c)}
                on:click={() => (hiddenCountries = toggle(hiddenCountries, c))}
              >
                {c} <span class="num">{n}</span>
              </button>
            {/each}
          </div>
        </div>
        {#if hiddenDomains.size || hiddenCountries.size}
          <button class="ghost small" on:click={() => { hiddenDomains = new Set(); hiddenCountries = new Set(); }}>
            Réinitialiser les filtres
          </button>
        {/if}
      </div>

      <p class="note">
        Heures en UTC. Pour GDELT, il s'agit du moment où l'article a été repéré, généralement proche de sa
        publication.
      </p>

      <!-- The timeline itself -->
      <ol class="days">
        {#each byDay as [day, list] (day)}
          <li class="day" id="day-{day}">
            <h3 class="day-title">
              <span class="day-name">{dayLabel(day)}</span>
              <span class="day-count num">{list.length}</span>
            </h3>
            <ol class="items">
              {#each list as a (a.id)}
                <li class="item">
                  <span class="time mono">{timeLabel(a.published_at)}</span>
                  <span class="dot" aria-hidden="true"></span>
                  <div class="body">
                    <a class="title" href={a.url} target="_blank" rel="noopener noreferrer">
                      {a.title}
                      <Icon name="external" size={12} />
                    </a>
                    <span class="meta">
                      <span class="mono">{a.domain}</span>
                      {#if a.source_country}<span class="badge">{a.source_country}</span>{/if}
                      {#if a.language}<span>{a.language}</span>{/if}
                    </span>
                  </div>
                </li>
              {/each}
            </ol>
          </li>
        {/each}
      </ol>
    {/if}
  {/if}
</section>

<style>
  .controls {
    gap: 0.6rem;
  }

  .field {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: var(--fs-xs);
    color: var(--text-2);
  }

  .field input {
    font: inherit;
    font-size: var(--fs-sm);
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 0.35rem 0.5rem;
  }

  .btn-inner {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }

  .alert {
    margin-bottom: 0.75rem;
  }

  .summary {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 1.25rem;
    font-size: var(--fs-sm);
    color: var(--text-2);
    margin: 0.25rem 0 0.75rem;
  }

  .summary strong {
    color: var(--text);
  }

  .warn-text {
    color: var(--warning);
    font-weight: 600;
  }

  /* ---- Volume chart ---- */
  .volume {
    padding: 0.75rem 0.85rem 0.5rem;
    margin-bottom: 0.9rem;
  }

  .bars {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 72px;
  }

  .bar {
    flex: 1;
    min-width: 2px;
    height: var(--h);
    padding: 0;
    border: none;
    border-radius: 2px 2px 0 0;
    background: var(--brand-text);
    opacity: 0.85;
  }

  .bar:hover:not(:disabled) {
    background: var(--brand-text);
    opacity: 1;
  }

  .bar.empty-bar {
    background: var(--border);
    opacity: 1;
    cursor: default;
  }

  .axis {
    display: flex;
    justify-content: space-between;
    margin-top: 0.35rem;
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  /* ---- Filters ---- */
  .filters {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .filter-row {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .filter-row .eyebrow {
    flex: 0 0 110px;
    padding-top: 0.35rem;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .chip .num {
    color: var(--text-3);
    margin-left: 0.2rem;
  }

  .note {
    margin: 0 0 1rem;
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  /* ---- Days ---- */
  .days,
  .items {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .day {
    margin-bottom: 1.5rem;
    scroll-margin-top: calc(var(--topbar-h) + 1rem);
  }

  .day-title {
    position: sticky;
    top: var(--topbar-h);
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin: 0 0 0.5rem;
    padding: 0.4rem 0;
    font-size: var(--fs-md);
    font-weight: 650;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }

  .day-name::first-letter {
    text-transform: uppercase;
  }

  .day-count {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-2);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 0.45rem;
  }

  .item {
    display: grid;
    grid-template-columns: 3.2rem 14px minmax(0, 1fr);
    gap: 0.6rem;
    align-items: start;
    padding: 0.45rem 0;
  }

  .time {
    color: var(--text-2);
    font-size: var(--fs-sm);
    padding-top: 0.05rem;
    text-align: right;
  }

  /* vertical rail with a dot per article */
  .dot {
    position: relative;
    align-self: stretch;
  }

  .dot::before {
    content: '';
    position: absolute;
    left: 6px;
    top: 0;
    bottom: -0.9rem;
    width: 2px;
    background: var(--border);
  }

  .dot::after {
    content: '';
    position: absolute;
    left: 2px;
    top: 0.35rem;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--surface);
    border: 2px solid var(--brand-text);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
  }

  .title {
    color: var(--text);
    font-weight: 500;
    text-decoration: none;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .title:hover {
    color: var(--brand-text);
    text-decoration: underline;
  }

  .title :global(svg) {
    display: inline;
    vertical-align: -1px;
    color: var(--text-3);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .empty {
    text-align: center;
    padding: 2.5rem 1.5rem;
    max-width: 620px;
    margin: 1.5rem auto;
  }

  .empty h2 {
    margin: 0 0 0.5rem;
    font-size: var(--fs-lg);
  }

  .empty p {
    color: var(--text-2);
    margin: 0 0 1.25rem;
  }

  .bars-skeleton {
    height: 110px;
  }

  @media (max-width: 640px) {
    .filter-row {
      flex-direction: column;
      gap: 0.35rem;
    }

    .filter-row .eyebrow {
      flex: none;
      padding-top: 0;
    }

    .item {
      grid-template-columns: 2.8rem 14px minmax(0, 1fr);
      gap: 0.45rem;
    }
  }
</style>
