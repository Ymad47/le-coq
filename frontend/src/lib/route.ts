// URL hash state: #/<view>?q=<query>   (#/ or no hash = home page)
// Keeps the current view and query shareable and restores them on reload.

import type { View } from './types';

const VIEWS: View[] = ['home', 'list', 'graph', 'focus', 'map'];

export interface RouteState {
  view: View;
  query: string;
}

export function readRoute(hash: string = window.location.hash): RouteState {
  const raw = hash.replace(/^#\/?/, '');
  const [path, qs = ''] = raw.split('?');
  const view = (VIEWS as string[]).includes(path) ? (path as View) : 'home';
  const query = new URLSearchParams(qs).get('q') ?? '';
  return { view, query };
}

export function routeHash(state: RouteState): string {
  const q = state.query.trim();
  const path = state.view === 'home' ? '' : state.view;
  return `#/${path}${q && state.view !== 'home' ? `?${new URLSearchParams({ q })}` : ''}`;
}

/** Push a history entry only when the route actually changes. */
export function writeRoute(state: RouteState): void {
  const next = routeHash(state);
  if (next !== window.location.hash) history.pushState(null, '', next);
}
