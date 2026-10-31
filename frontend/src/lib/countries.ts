// Country normalization for the world map.
// The jsvectormap `world_merc` map keys its regions by ISO 3166-1 alpha-2
// (FR, US…). Plugins return countries in three shapes: ISO-3 codes
// (gdelt, uncomtrade), ISO-2 codes (gleif) and full names (eurostat, acled).
// Everything is normalized to ISO-2 here.

import type { Entity } from './types';

/** ISO-3 → ISO-2 for every region of world_merc (plus common extras). */
export const ISO3_TO_2: Record<string, string> = {
  AFG: 'AF', AGO: 'AO', ALB: 'AL', ARE: 'AE', ARG: 'AR', ARM: 'AM', ATF: 'TF', AUS: 'AU',
  AUT: 'AT', AZE: 'AZ', BDI: 'BI', BEL: 'BE', BEN: 'BJ', BFA: 'BF', BGD: 'BD', BGR: 'BG',
  BHS: 'BS', BIH: 'BA', BLR: 'BY', BLZ: 'BZ', BOL: 'BO', BRA: 'BR', BRN: 'BN', BTN: 'BT',
  BWA: 'BW', CAF: 'CF', CAN: 'CA', CHE: 'CH', CHL: 'CL', CHN: 'CN', CIV: 'CI', CMR: 'CM',
  COD: 'CD', COG: 'CG', COL: 'CO', CRI: 'CR', CUB: 'CU', CYP: 'CY', CZE: 'CZ', DEU: 'DE',
  DJI: 'DJ', DNK: 'DK', DOM: 'DO', DZA: 'DZ', ECU: 'EC', EGY: 'EG', ERI: 'ER', ESH: 'EH',
  ESP: 'ES', EST: 'EE', ETH: 'ET', FIN: 'FI', FJI: 'FJ', FLK: 'FK', FRA: 'FR', GAB: 'GA',
  GBR: 'GB', GEO: 'GE', GHA: 'GH', GIN: 'GN', GMB: 'GM', GNB: 'GW', GNQ: 'GQ', GRC: 'GR',
  GRL: 'GL', GTM: 'GT', GUY: 'GY', HND: 'HN', HRV: 'HR', HTI: 'HT', HUN: 'HU', IDN: 'ID',
  IND: 'IN', IRL: 'IE', IRN: 'IR', IRQ: 'IQ', ISL: 'IS', ISR: 'IL', ITA: 'IT', JAM: 'JM',
  JOR: 'JO', JPN: 'JP', KAZ: 'KZ', KEN: 'KE', KGZ: 'KG', KHM: 'KH', KOR: 'KR', KWT: 'KW',
  LAO: 'LA', LBN: 'LB', LBR: 'LR', LBY: 'LY', LKA: 'LK', LSO: 'LS', LTU: 'LT', LUX: 'LU',
  LVA: 'LV', MAR: 'MA', MDA: 'MD', MDG: 'MG', MEX: 'MX', MKD: 'MK', MLI: 'ML', MMR: 'MM',
  MNE: 'ME', MNG: 'MN', MOZ: 'MZ', MRT: 'MR', MWI: 'MW', MYS: 'MY', NAM: 'NA', NCL: 'NC',
  NER: 'NE', NGA: 'NG', NIC: 'NI', NLD: 'NL', NOR: 'NO', NPL: 'NP', NZL: 'NZ', OMN: 'OM',
  PAK: 'PK', PAN: 'PA', PER: 'PE', PHL: 'PH', PNG: 'PG', POL: 'PL', PRI: 'PR', PRK: 'KP',
  PRT: 'PT', PRY: 'PY', PSE: 'PS', QAT: 'QA', ROU: 'RO', RUS: 'RU', RWA: 'RW', SAU: 'SA',
  SDN: 'SD', SEN: 'SN', SLB: 'SB', SLE: 'SL', SLV: 'SV', SOM: 'SO', SRB: 'RS', SSD: 'SS',
  SUR: 'SR', SVK: 'SK', SVN: 'SI', SWE: 'SE', SWZ: 'SZ', SYR: 'SY', TCD: 'TD', TGO: 'TG',
  THA: 'TH', TJK: 'TJ', TKM: 'TM', TLS: 'TL', TTO: 'TT', TUN: 'TN', TUR: 'TR', TWN: 'TW',
  TZA: 'TZ', UGA: 'UG', UKR: 'UA', URY: 'UY', USA: 'US', UZB: 'UZ', VEN: 'VE', VNM: 'VN',
  VUT: 'VU', XKX: 'XK', YEM: 'YE', ZAF: 'ZA', ZMB: 'ZM', ZWE: 'ZW',
};

/** Common spellings that differ from the map's own region names (lowercase keys). */
const NAME_ALIASES: Record<string, string> = {
  'united states of america': 'US', usa: 'US', 'u.s.': 'US', 'états-unis': 'US',
  uk: 'GB', 'great britain': 'GB', 'royaume-uni': 'GB',
  'russian federation': 'RU', russie: 'RU',
  'czech republic': 'CZ', czechia: 'CZ',
  'bosnia and herzegovina': 'BA',
  'democratic republic of the congo': 'CD', 'dr congo': 'CD',
  'republic of the congo': 'CG',
  'central african republic': 'CF',
  'dominican republic': 'DO',
  'ivory coast': 'CI', "cote d'ivoire": 'CI',
  'south korea': 'KR', 'republic of korea': 'KR',
  'north korea': 'KP',
  laos: 'LA',
  'north macedonia': 'MK',
  eswatini: 'SZ',
  'south sudan': 'SS',
  'western sahara': 'EH',
  'equatorial guinea': 'GQ',
  'solomon islands': 'SB',
  'falkland islands': 'FK',
  'east timor': 'TL',
  'state of palestine': 'PS',
  allemagne: 'DE', espagne: 'ES', italie: 'IT', chine: 'CN', 'royaume uni': 'GB',
};

const mapNameToIso2 = new Map<string, string>();
const knownIso2 = new Set<string>(Object.values(ISO3_TO_2));

/** Register the map's own region names (called once the map file is loaded). */
export function registerRegionNames(paths: Record<string, { name?: string }>): void {
  for (const [code, data] of Object.entries(paths)) {
    knownIso2.add(code);
    if (data?.name) mapNameToIso2.set(data.name.toLowerCase(), code);
  }
}

function fromName(name: unknown): string | null {
  if (typeof name !== 'string' || !name.trim()) return null;
  const key = name.trim().toLowerCase();
  return NAME_ALIASES[key] ?? mapNameToIso2.get(key) ?? null;
}

function fromCode(code: unknown): string | null {
  if (typeof code !== 'string') return null;
  const c = code.trim().toUpperCase();
  if (c.length === 2 && knownIso2.has(c)) return c;
  if (c.length === 3) return ISO3_TO_2[c] ?? null;
  return null;
}

/** Best-effort ISO-2 code for an entity, or null when it has no country. */
export function entityIso2(e: Entity): string | null {
  const p = e.properties ?? {};
  return (
    fromCode(p.iso2) ??
    fromCode(p.iso3) ??
    fromCode(p.iso3_code) ??
    fromCode(p.country_code) ??
    // gleif puts an ISO-2 code in `country`; eurostat/acled put a name there
    fromCode(p.country) ??
    fromName(p.country) ??
    fromName(e.country) ??
    (e.id?.startsWith('eurostat_country_')
      ? fromCode(e.id.slice('eurostat_country_'.length).split('_')[0])
      : null) ??
    (e.id?.startsWith('uncomtrade_country_')
      ? fromName(e.id.slice('uncomtrade_country_'.length).replace(/_/g, ' '))
      : null) ??
    // Country entities themselves (entity_type "Country") carry the name
    ((e.entity_type ?? '').toLowerCase() === 'country' ? fromName(e.name) : null)
  );
}
