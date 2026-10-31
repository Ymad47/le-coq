<script lang="ts" context="module">
  // Minimal line icons (24×24 grid, 1.6 stroke). Kept in-house: the app only
  // needs a handful, which does not justify an icon library dependency.
  const PATHS = {
    search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14Zm9 16-4.35-4.35',
    home: 'M4 10.5 12 4l8 6.5V20a1 1 0 0 1-1 1h-4.5v-6h-5v6H5a1 1 0 0 1-1-1v-9.5Z',
    list: 'M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01',
    graph: 'M6 6.5 12 12m0 0 6-5.5M12 12v6 M6 4.75a1.75 1.75 0 1 0 0 3.5 1.75 1.75 0 0 0 0-3.5ZM18 4.75a1.75 1.75 0 1 0 0 3.5 1.75 1.75 0 0 0 0-3.5ZM12 10.25a1.75 1.75 0 1 0 0 3.5 1.75 1.75 0 0 0 0-3.5ZM12 18a1.75 1.75 0 1 0 0 3.5 1.75 1.75 0 0 0 0-3.5Z',
    focus: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm0 4.5a4.5 4.5 0 1 0 0 9 4.5 4.5 0 0 0 0-9Zm0 3.5a1 1 0 1 0 0 2 1 1 0 0 0 0-2Z',
    map: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm-9 9h18M12 3c2.5 2.5 3.5 5.5 3.5 9s-1 6.5-3.5 9c-2.5-2.5-3.5-5.5-3.5-9s1-6.5 3.5-9Z',
    sun: 'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm0-5v2m0 14v2M3 12h2m14 0h2M5.6 5.6 7 7m10 10 1.4 1.4M5.6 18.4 7 17M17 7l1.4-1.4',
    moon: 'M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5Z',
    close: 'M6 6l12 12M18 6 6 18',
    reload: 'M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7',
    arrowRight: 'M5 12h14m-6-6 6 6-6 6',
    arrowLeft: 'M19 12H5m6-6-6 6 6 6',
    external: 'M14 4h6v6m0-6-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5',
    alert: 'M12 4 2.5 20h19L12 4Zm0 6v4m0 3h.01',
    info: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm0 8v5m0-8h.01',
    expand: 'M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5',
    globe: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm-9 9h18',
  } as const;

  export type IconName = keyof typeof PATHS;
</script>

<script lang="ts">
  export let name: IconName;
  export let size = 18;
  /** Accessible label; omit for decorative icons next to visible text. */
  export let label = '';
</script>

<svg
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  role={label ? 'img' : undefined}
  aria-label={label || undefined}
  aria-hidden={label ? undefined : 'true'}
  focusable="false"
>
  <path d={PATHS[name]} />
</svg>

<style>
  svg {
    flex-shrink: 0;
    display: block;
  }
</style>
