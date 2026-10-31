// Light / dark theme. index.html sets <html data-theme> before first paint
// (no flash); this store keeps it in sync and remembers an explicit choice.
// Without a saved choice, the theme follows the OS preference live.

import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark';

const STORAGE_KEY = 'lecoq-theme';

function saved(): Theme | null {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return v === 'light' || v === 'dark' ? v : null;
  } catch {
    return null; // storage blocked (private mode, policy…)
  }
}

const media = window.matchMedia('(prefers-color-scheme: dark)');
const system = (): Theme => (media.matches ? 'dark' : 'light');

export const theme = writable<Theme>(saved() ?? system());

theme.subscribe((t) => {
  document.documentElement.dataset.theme = t;
});

media.addEventListener('change', () => {
  if (!saved()) theme.set(system());
});

export function toggleTheme(): void {
  theme.update((t) => {
    const next: Theme = t === 'dark' ? 'light' : 'dark';
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      /* not persisted, still applied for this session */
    }
    return next;
  });
}
