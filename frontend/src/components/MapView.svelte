<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { createEventDispatcher } from 'svelte';
  import type { Entity, MapCountryInfo } from '../lib/types';
  import { entityIso2, registerRegionNames } from '../lib/countries';
  import { vizPalette, mixHex } from '../lib/tokens';
  import { loadWorldMerc } from '../lib/worldMap';
  // Structural styles (tooltip show/hide, zoom buttons, container); colors are
  // remapped to our tokens in app.css (--jvm-* variables).
  import 'jsvectormap/dist/jsvectormap.css';

  export let entities: Entity[] = [];

  const dispatch = createEventDispatcher<{
    countrySelected: MapCountryInfo;
    searchCountry: { query: string };
    ingestCountry: { query: string };
  }>();

  let mapEl: HTMLDivElement;
  let worldMap: any = null;
  let error = '';
  let mappedCount = 0;


  export async function renderMap(): Promise<void> {
    if (!mapEl) return;
    try {
      const { jsVectorMap, worldMerc } = await loadWorldMerc();
      // Region names must be known before matching entities by country name
      registerRegionNames(worldMerc.paths ?? {});

      // Map regions are keyed by ISO-2 (FR, US…)
      const counts = new Map<string, number>();
      for (const e of entities) {
        const iso = entityIso2(e);
        if (!iso) continue;
        counts.set(iso, (counts.get(iso) ?? 0) + 1);
      }
      mappedCount = [...counts.values()].reduce((a, b) => a + b, 0);
      worldMap?.destroy?.();
      const values = Object.fromEntries(counts);
      const max = Math.max(...counts.values(), 1);

      const regionNames = new Map<string, string>();

      // Colors come from the active theme's tokens
      const palette = vizPalette();
      worldMap = new jsVectorMap({
        selector: mapEl,
        map: 'world_merc',
        backgroundColor: 'transparent',
        regionStyle: {
          initial: { fill: palette.mapEmpty, stroke: palette.mapStroke, strokeWidth: 0.5 },
          hover: { fill: palette.mapHover, fillOpacity: 1, stroke: palette.mapStroke },
          selected: { fill: palette.brandText, fillOpacity: 1 },
        },
        series: {
          // jsvectormap 1.7 only supports ordinal scales (value -> color) and
          // ignores min/max/normalizeFunction, so the gradient is computed here.
          // Log scale: entity counts per country are very skewed.
          regions: [{
            values,
            scale: Object.fromEntries(
              [...new Set(counts.values())].map((v) => [
                v,
                mixHex(palette.mapLow, palette.mapHigh, max > 1 ? Math.log1p(v) / Math.log1p(max) : 1),
              ])
            ),
          }],
        },
        onRegionTooltipShow: (evt: any, tooltip: any, code: string) => {
          const n = counts.get(code) ?? 0;
          const name = regionNames.get(code) ?? code;
          tooltip.text(`${name} — ${n} entité${n !== 1 ? 's' : ''}`, true);
        },
        onRegionSelected: (_evt: any, code: string, isSelected: boolean, _sel: any) => {
          if (!isSelected) return;
          const n = counts.get(code) ?? 0;
          const name = regionNames.get(code) ?? code;
          if (n === 0) {
            dispatch('countrySelected', { code, name, entities: [] });
            return;
          }
          const countryEntities = entities.filter((e) => entityIso2(e) === code);
          dispatch('countrySelected', { code, name, entities: countryEntities, count: n });
        },
      });

      try {
        const paths = worldMerc.paths ?? {};
        for (const [code, data] of Object.entries(paths)) {
          if ((data as any)?.name) regionNames.set(code, (data as any).name);
        }
      } catch (e) {
        console.warn('[map] could not preload region names', e);
      }
    } catch (e: any) {
      error = `Render map: ${e.message}`;
      console.error('[MapView]', e);
    }
  }

  export function getError(): string {
    return error;
  }

  onDestroy(() => {
    worldMap?.destroy?.();
  });
</script>

<div class="view-toolbar">
  <span class="stats">
    {mappedCount} / {entities.length} entités localisées · cliquer sur un pays pour le détail
  </span>
</div>
{#if error}<div class="alert error">{error}</div>{/if}

<div class="map-wrap" bind:this={mapEl}></div>

<style>
  .map-wrap {
    position: relative;
    overflow: hidden;
    height: calc(100vh - var(--topbar-h) - 140px);
    min-height: 420px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
  }


  @media (max-width: 820px) {
    .map-wrap {
      height: 55vh;
      min-height: 300px;
    }
  }
</style>
