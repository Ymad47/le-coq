<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { createEventDispatcher } from 'svelte';
  import type { Entity, MapCountryInfo } from '../lib/types';
  import { entityIso2, registerRegionNames } from '../lib/countries';

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
      const mod = await import('jsvectormap');
      await import('jsvectormap/dist/maps/world-merc.js');
      const jsVectorMap = (mod as any).default ?? (window as any).jsVectorMap;
      if (!jsVectorMap) { error = 'jsvectormap non chargé'; return; }
      // Region names must be known before matching entities by country name
      registerRegionNames((jsVectorMap as any).maps?.world_merc?.paths ?? {});

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

      worldMap = new jsVectorMap({
        selector: mapEl,
        map: 'world_merc',
        backgroundColor: 'transparent',
        regionStyle: {
          initial: { fill: 'rgba(255,255,255,0.06)', stroke: 'rgba(255,255,255,0.12)', strokeWidth: 0.4 },
          hover: { fill: '#14b8a6', fillOpacity: 0.75, stroke: 'rgba(255,255,255,0.3)' },
          selected: { fill: '#2dd4bf', fillOpacity: 0.95 },
        },
        series: {
          regions: [{
            values,
            scale: ['#1f2937', '#2dd4bf'],
            normalizeFunction: 'linear',
            min: 0,
            max,
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
        const paths = (jsVectorMap as any).maps?.world_merc?.paths ?? {};
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
    height: 70vh;
    min-height: 400px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    backdrop-filter: blur(20px);
  }
</style>
