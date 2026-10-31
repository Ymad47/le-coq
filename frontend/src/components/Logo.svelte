<script lang="ts">
  /**
   * Le Coq brand mark: geometric rooster head in an open ring, French Blue crest.
   * Vector traced from the brand board (no original SVG in the repository).
   *
   * variant:
   *   symbol      mark only (favicons, avatars, tight spaces)
   *   horizontal  mark + "Le Coq"
   *   full        mark + "Le Coq" + descriptor line
   *   compact     navigation size: mark + "Le Coq", tighter spacing
   * tone:    brand (blue crest) | mono (single color)
   * surface: auto (follows the theme) | dark (forces the light silhouette,
   *          for dark imagery) | light (forces the navy silhouette)
   */
  export let variant: 'symbol' | 'horizontal' | 'full' | 'compact' = 'horizontal';
  export let tone: 'brand' | 'mono' = 'brand';
  export let surface: 'auto' | 'dark' | 'light' = 'auto';
  /** Mark height in px; the wordmark scales with it. */
  export let size = 36;
  export let descriptor = 'Geopolitical Intelligence Platform';
  /** Accessible name. Empty string = decorative. */
  export let label = 'Le Coq';

  $: showName = variant !== 'symbol';
  $: showDescriptor = variant === 'full';
</script>

<span
  class="logo {variant}"
  class:surface-dark={surface === 'dark'}
  class:surface-light={surface === 'light'}
  class:mono={tone === 'mono'}
  style="--size: {size}px"
  role={label ? 'img' : undefined}
  aria-label={label || undefined}
  aria-hidden={label ? undefined : 'true'}
>
  <svg viewBox="20 36 740 740" width={size} height={size} aria-hidden="true" focusable="false">
    <g class="fg-stroke" fill="none" stroke-width="58">
      <path d="M430 72 A335 335 0 0 1 587 676" />
      <path d="M350 72 A335 335 0 0 0 81 534" />
    </g>
    <path class="crest" d="M250 195 L372 222 L360 132 L470 178 L508 250 L516 345 L470 302 L332 300 Z" />
    <g class="fg-fill">
      <path
        d="M115 522 C150 420 230 345 330 325 C400 312 455 318 495 350 C440 345 375 352 320 385 C275 415 255 470 252 545 L258 712 C195 660 145 600 115 522 Z"
      />
      <path d="M437 440 L495 372 L585 470 Z" />
      <path d="M403 503 L468 453 L518 712 Z" />
      <circle cx="418" cy="380" r="16" />
    </g>
  </svg>

  {#if showName}
    <span class="text" aria-hidden="true">
      <span class="name">Le Coq</span>
      {#if showDescriptor}<span class="descriptor">{descriptor}</span>{/if}
    </span>
  {/if}
</span>

<style>
  .logo {
    --fg: var(--logo-fg);
    --crest: var(--logo-crest);
    display: inline-flex;
    align-items: center;
    gap: calc(var(--size) * 0.32);
    color: var(--fg);
    line-height: 1;
  }

  .logo.surface-dark {
    --fg: var(--off-white);
    --crest: var(--crest-on-dark);
  }

  .logo.surface-light {
    --fg: var(--deep-navy);
    --crest: var(--french-blue);
  }

  .logo.mono {
    --crest: var(--fg);
  }

  .logo.compact {
    gap: calc(var(--size) * 0.26);
  }

  svg {
    flex-shrink: 0;
    display: block;
  }

  .fg-stroke {
    stroke: var(--fg);
  }

  .fg-fill {
    fill: var(--fg);
  }

  .crest {
    fill: var(--crest);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: calc(var(--size) * 0.14);
  }

  .name {
    font-weight: 700;
    font-size: calc(var(--size) * 0.6);
    letter-spacing: -0.025em;
  }

  .compact .name {
    font-size: calc(var(--size) * 0.5);
    font-weight: 650;
  }

  .descriptor {
    font-size: max(10px, calc(var(--size) * 0.155));
    font-weight: 500;
    letter-spacing: var(--tracking-caps);
    text-transform: uppercase;
    color: var(--text-2);
  }

  .surface-dark .descriptor {
    color: var(--slate);
  }
</style>
