<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { createEventDispatcher } from 'svelte';
  import type { Entity, MapCountryInfo } from '../lib/types';

  export let entities: Entity[] = [];

  const dispatch = createEventDispatcher<{
    countrySelected: MapCountryInfo;
    searchCountry: { query: string };
    ingestCountry: { query: string };
  }>();

  let mapEl: HTMLDivElement;
  let worldMap: any = null;
  let error = '';

  const NAME_TO_ISO: Record<string, string> = {
    'France': 'FRA', 'United States': 'USA', 'United States of America': 'USA',
    'China': 'CHN', 'Germany': 'DEU', 'United Kingdom': 'GBR', 'UK': 'GBR',
    'Russia': 'RUS', 'Brazil': 'BRA', 'India': 'IND', 'Japan': 'JPN',
    'South Africa': 'ZAF', 'Canada': 'CAN', 'Mexico': 'MEX', 'Australia': 'AUS',
    'South Korea': 'KOR', 'Korea': 'KOR',
    'Spain': 'ESP', 'Italy': 'ITA', 'Belgium': 'BEL', 'Netherlands': 'NLD',
    'Poland': 'POL', 'Turkey': 'TUR', 'Saudi Arabia': 'SAU',
    'United Arab Emirates': 'ARE', 'Qatar': 'QAT',
    'Egypt': 'EGY', 'Morocco': 'MAR', 'Algeria': 'DZA', 'Nigeria': 'NGA',
    'Kenya': 'KEN', 'Ghana': 'GHA', 'Ethiopia': 'ETH',
    'Switzerland': 'CHE', 'Austria': 'AUT', 'Denmark': 'DNK', 'Finland': 'FIN',
    'Greece': 'GRC', 'Ireland': 'IRL', 'Norway': 'NOR', 'Portugal': 'PRT',
    'Sweden': 'SWE', 'Czech Republic': 'CZE', 'Hungary': 'HUN', 'Romania': 'ROU',
    'Bulgaria': 'BGR', 'Croatia': 'HRV', 'Slovakia': 'SVK', 'Slovenia': 'SVN',
    'Estonia': 'EST', 'Latvia': 'LVA', 'Lithuania': 'LTU', 'Ukraine': 'UKR',
    'Israel': 'ISR', 'Iran': 'IRN', 'Iraq': 'IRQ', 'Jordan': 'JOR',
    'Lebanon': 'LBN', 'Syria': 'SYR', 'Yemen': 'YEM', 'Oman': 'OMN',
    'Bahrain': 'BHR', 'Kuwait': 'KWT', 'New Zealand': 'NZL',
    'Argentina': 'ARG', 'Chile': 'CHL', 'Colombia': 'COL', 'Peru': 'PER',
    'Venezuela': 'VEN', 'Pakistan': 'PAK', 'Bangladesh': 'BGD',
    'Philippines': 'PHL', 'Indonesia': 'IDN', 'Malaysia': 'MYS',
    'Singapore': 'SGP', 'Thailand': 'THA', 'Vietnam': 'VNM',
    'Taiwan': 'TWN', 'Hong Kong': 'HKG',
  };

  const ISO2_TO_3: Record<string, string> = {
    FR: 'FRA', DE: 'DEU', IT: 'ITA', ES: 'ESP', GB: 'GBR', NL: 'NLD', BE: 'BEL',
    PL: 'POL', PT: 'PRT', AT: 'AUT', CH: 'CHE', SE: 'SWE', NO: 'NOR', DK: 'DNK',
    FI: 'FIN', GR: 'GRC', IE: 'IRL', RO: 'ROU', CZ: 'CZE', HU: 'HUN', SK: 'SVK',
    SI: 'SVN', BG: 'BGR', HR: 'HRV', EE: 'EST', LV: 'LVA', LT: 'LTU', LU: 'LUX',
    MT: 'MLT', CY: 'CYP',
  };

  function entityToIso(e: Entity): string | null {
    let iso = e.properties?.iso3 ?? e.properties?.iso3_code;
    if (!iso && e.id?.startsWith('uncomtrade_country_')) {
      const name = e.id.slice('uncomtrade_country_'.length).replace(/_/g, ' ');
      iso = NAME_TO_ISO[name];
    }
    if (!iso && e.id?.startsWith('eurostat_country_')) {
      const two = e.id.slice('eurostat_country_'.length).split('_')[0]?.toUpperCase();
      iso = ISO2_TO_3[two ?? ''];
    }
    return iso && /^[A-Z]{3}$/.test(iso) ? iso : null;
  }

  export async function renderMap(): Promise<void> {
    if (!mapEl) return;
    try {
      const counts = new Map<string, number>();
      for (const e of entities) {
        const iso = entityToIso(e);
        if (!iso) continue;
        counts.set(iso, (counts.get(iso) ?? 0) + 1);
      }

      const mod = await import('jsvectormap');
      await import('jsvectormap/dist/maps/world-merc.js');
      const jsVectorMap = (mod as any).default ?? (window as any).jsVectorMap;
      if (!jsVectorMap) { error = 'jsvectormap non chargé'; return; }
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
          const countryEntities = entities.filter((e) => entityToIso(e) === code);
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

<div class="map-toolbar">
  <span class="graph-stats">🗺️ Carte — densité d'entités par pays · cliquer pour voir les détails</span>
</div>

<div class="map-wrap" bind:this={mapEl}></div>

<style>
  .map-toolbar {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    margin-bottom: 1rem;
    padding: 0.75rem;
    background: rgba(255, 255, 255, 0.03);
    backdrop-filter: blur(20px) saturate(180%);
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .graph-stats {
    color: #94a3b8;
    font-size: 0.85rem;
  }

  .map-wrap {
    height: 70vh;
    min-height: 400px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    backdrop-filter: blur(20px);
  }
</style>
