<script lang="ts">
  /**
   * Home-page globe: country outlines, a few capitals and static great-circle
   * "signal" arcs from Paris, rotating very slowly. Canvas + d3-geo.
   * Colors come from the active theme (lib/tokens.ts) and update on switch.
   * Pauses when off-screen or when the tab is hidden; does not rotate when
   * the user prefers reduced motion.
   */
  import { onMount } from 'svelte';
  import { geoOrthographic, geoPath, geoGraticule10, geoDistance } from 'd3-geo';
  import { loadWorldLines, type LonLat } from '../lib/worldLines';
  import { vizPalette } from '../lib/tokens';
  import { theme } from '../lib/theme';

  /** Degrees per second. 1.5 → one full turn every 4 minutes. */
  export let speed = 1.5;

  let canvas: HTMLCanvasElement;
  let wrap: HTMLDivElement;

  const CITIES: Record<string, LonLat> = {
    paris: [2.35, 48.86],
    brussels: [4.35, 50.85],
    kyiv: [30.52, 50.45],
    moscow: [37.62, 55.75],
    cairo: [31.24, 30.04],
    tehran: [51.39, 35.69],
    delhi: [77.21, 28.61],
    beijing: [116.4, 39.9],
    tokyo: [139.69, 35.69],
    dakar: [-17.45, 14.69],
    bamako: [-8.0, 12.64],
    nairobi: [36.82, -1.29],
    johannesburg: [28.05, -26.2],
    washington: [-77.04, 38.9],
    brasilia: [-47.88, -15.79],
  };

  const LINKS: [string, string][] = [
    ['paris', 'kyiv'],
    ['paris', 'cairo'],
    ['paris', 'dakar'],
    ['paris', 'bamako'],
    ['paris', 'washington'],
    ['paris', 'delhi'],
    ['paris', 'nairobi'],
    ['brussels', 'moscow'],
    ['moscow', 'beijing'],
    ['washington', 'brasilia'],
  ];

  onMount(() => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const projection = geoOrthographic().clipAngle(90).precision(0.5);
    const path = geoPath(projection, ctx);
    const graticule = geoGraticule10();
    const arcs = LINKS.map(([a, b]) => ({ type: 'LineString', coordinates: [CITIES[a], CITIES[b]] }) as any);
    let world: Awaited<ReturnType<typeof loadWorldLines>> | null = null;
    let palette = vizPalette();

    // Europe / Africa facing the viewer at start
    let lambda = -15;
    const phi = -25;
    let size = 0;
    let dpr = 1;
    let raf = 0;
    let last = 0;
    let visible = true;

    function resize() {
      const rect = wrap.getBoundingClientRect();
      size = Math.max(160, Math.min(rect.width, rect.height));
      dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.round(size * dpr);
      canvas.height = Math.round(size * dpr);
      canvas.style.width = `${size}px`;
      canvas.style.height = `${size}px`;
      projection.translate([size / 2, size / 2]).scale(size * 0.46);
      draw();
    }

    function facing(p: LonLat): boolean {
      // Visible hemisphere: within 90° of the point under the center
      return geoDistance(p, [-lambda, -phi]) < Math.PI / 2 - 0.03;
    }

    function draw() {
      if (!ctx || !size) return;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, size, size);
      projection.rotate([lambda, phi]);
      const c = size / 2;
      const r = projection.scale();

      // Sphere
      ctx.beginPath();
      ctx.arc(c, c, r, 0, Math.PI * 2);
      ctx.fillStyle = palette.globeSea;
      ctx.fill();

      // Graticule
      ctx.beginPath();
      path(graticule);
      ctx.strokeStyle = palette.globeGrid;
      ctx.lineWidth = 0.6;
      ctx.stroke();

      // Country outlines
      if (world) {
        ctx.beginPath();
        path(world as any);
        ctx.strokeStyle = palette.globeLand;
        ctx.lineWidth = 0.7;
        ctx.stroke();
      }

      // Signal arcs (static)
      ctx.beginPath();
      for (const a of arcs) path(a);
      ctx.strokeStyle = palette.globeArc;
      ctx.lineWidth = 1;
      ctx.stroke();

      // Capitals
      ctx.fillStyle = palette.globeCity;
      for (const p of Object.values(CITIES)) {
        if (!facing(p)) continue;
        const xy = projection(p);
        if (!xy) continue;
        ctx.beginPath();
        ctx.arc(xy[0], xy[1], p === CITIES.paris ? 3 : 1.8, 0, Math.PI * 2);
        ctx.fill();
      }

      // Rim
      ctx.beginPath();
      ctx.arc(c, c, r, 0, Math.PI * 2);
      ctx.strokeStyle = palette.globeRim;
      ctx.lineWidth = 1;
      ctx.stroke();
    }

    function frame(now: number) {
      const dt = last ? Math.min((now - last) / 1000, 0.1) : 0;
      last = now;
      lambda += speed * dt;
      draw();
      raf = requestAnimationFrame(frame);
    }

    function start() {
      if (reduceMotion || raf || !visible || document.hidden) return;
      last = 0;
      raf = requestAnimationFrame(frame);
    }

    function stop() {
      cancelAnimationFrame(raf);
      raf = 0;
    }

    const ro = new ResizeObserver(resize);
    ro.observe(wrap);
    const io = new IntersectionObserver(([entry]) => {
      visible = entry.isIntersecting;
      visible ? start() : stop();
    });
    io.observe(wrap);
    const onVisibility = () => (document.hidden ? stop() : start());
    document.addEventListener('visibilitychange', onVisibility);
    // Re-read colors after a theme switch (the attribute is already updated)
    const unsubscribe = theme.subscribe(() => {
      palette = vizPalette();
      draw();
    });

    resize();
    loadWorldLines()
      .then((w) => {
        world = w;
        draw();
      })
      .catch((e) => console.warn('[globe] outlines unavailable', e));
    start();

    return () => {
      stop();
      ro.disconnect();
      io.disconnect();
      unsubscribe();
      document.removeEventListener('visibilitychange', onVisibility);
    };
  });
</script>

<div class="globe" bind:this={wrap} aria-hidden="true">
  <canvas bind:this={canvas}></canvas>
</div>

<style>
  .globe {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
  }

  canvas {
    display: block;
  }
</style>
