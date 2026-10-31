<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { createEventDispatcher } from 'svelte';
  import cytoscape from 'cytoscape';
  import type { Entity, Relationship, GraphNode } from '../lib/types';
  import { colorForType, colorForEdge, extractYear, stripSourcePrefix } from '../lib/utils';

  export let entities: Entity[] = [];
  export let relationships: Relationship[] = [];
  export let focusHubId: string | null = null;
  export let initialLayout: 'cose' | 'breadthfirst' | 'concentric' | 'circle' = 'cose';
  /** True when the backend capped the number of relations returned. */
  export let truncated = false;
  /** Short description of what the graph shows (e.g. "voisinage de 12 résultats"). */
  export let scopeLabel = '';

  const dispatch = createEventDispatcher<{
    nodeSelected: GraphNode;
    nodeCleared: void;
    enrichFromNode: { query: string };
    exploreFromNode: { query: string };
  }>();

  let graphEl: HTMLDivElement;
  let cy: cytoscape.Core | null = null;
  let selectedNode: GraphNode | null = null;
  let graphLayout: string = initialLayout;
  let nodeCount = 0;
  let edgeCount = 0;
  let showIsolated = false;
  let hideWeakLinks = true;
  let maxYear = new Date().getFullYear();
  const MIN_YEAR = 2000;

  const qidLabelCache = new Map<string, string>();

  async function resolveQidLabels(ids: string[]): Promise<void> {
    const unknown = ids.filter((id) => id.startsWith('wd_item_') && !qidLabelCache.has(id));
    if (!unknown.length) return;
    const BATCH = 50;
    for (let i = 0; i < unknown.length; i += BATCH) {
      const chunk = unknown.slice(i, i + BATCH);
      const qids = chunk.map((id) => id.slice(8)).join('|');
      try {
        const url = `https://www.wikidata.org/w/api.php?action=wbgetentities&ids=${qids}&props=labels&languages=fr|en&format=json&origin=*`;
        const resp = await fetch(url);
        const data = await resp.json();
        for (const id of chunk) {
          const qid = id.slice(8);
          const ent = data.entities?.[qid];
          const label = ent?.labels?.fr?.value ?? ent?.labels?.en?.value;
          if (label) qidLabelCache.set(id, label);
        }
      } catch {
        // network fail: keep Q-code labels
      }
    }
  }

  function stableRandom(str: string): number {
    let h = 2166136261;
    for (let i = 0; i < str.length; i++) {
      h ^= str.charCodeAt(i);
      h = Math.imul(h, 16777619);
    }
    return ((h >>> 0) % 1000) / 1000;
  }

  function hslToHex(h: number, s: number, l: number): string {
    const c = (1 - Math.abs(2 * l - 1)) * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = l - c / 2;
    let r = 0, g = 0, b = 0;
    if (h < 60) { r = c; g = x; b = 0; }
    else if (h < 120) { r = x; g = c; b = 0; }
    else if (h < 180) { r = 0; g = c; b = x; }
    else if (h < 240) { r = 0; g = x; b = c; }
    else if (h < 300) { r = x; g = 0; b = c; }
    else { r = c; g = 0; b = x; }
    const c2 = (v: number) => Math.round((v + m) * 255).toString(16).padStart(2, '0');
    return `#${c2(r)}${c2(g)}${c2(b)}`;
  }

  function assignClusterColors(
    nodes: any[],
    edges: any[],
    deg: Map<string, number>,
    _maxDegree: number,
    topRatio = 0.08
  ): { color: Map<string, string>; hubIds: Set<string> } {
    const sorted = [...nodes].sort((a, b) => (deg.get(b.data.id) ?? 0) - (deg.get(a.data.id) ?? 0));
    const hubCount = Math.max(3, Math.ceil(nodes.length * topRatio));
    const hubs = sorted.filter((n) => (deg.get(n.data.id) ?? 0) > 0).slice(0, hubCount);

    const color = new Map<string, string>();
    const hubHue = new Map<string, number>();

    hubs.forEach((hub, i) => {
      const base = stableRandom(hub.data.id) * 360;
      const hue = Math.round((base + i * 137.508) % 360);
      hubHue.set(hub.data.id, hue);
      color.set(hub.data.id, hslToHex(hue, 0.75, 0.55));
    });

    const adj = new Map<string, string[]>();
    const push = (a: string, b: string) => {
      if (!adj.has(a)) adj.set(a, []);
      adj.get(a)!.push(b);
    };
    edges.forEach((e) => {
      push(e.data.source, e.data.target);
      push(e.data.target, e.data.source);
    });

    const queue = hubs.map((h) => ({ id: h.data.id, depth: 0, hue: hubHue.get(h.data.id)! }));
    const visited = new Set(hubs.map((h) => h.data.id));
    while (queue.length) {
      const { id, depth, hue } = queue.shift()!;
      color.set(id, hslToHex(hue, Math.max(0.25, 0.7 - depth * 0.14), Math.max(0.35, 0.62 - depth * 0.07)));
      for (const nb of adj.get(id) ?? []) {
        if (!visited.has(nb)) {
          visited.add(nb);
          queue.push({ id: nb, depth: depth + 1, hue });
        }
      }
    }

    nodes.forEach((n) => {
      if (!color.has(n.data.id)) color.set(n.data.id, '#2d3748');
    });

    return { color, hubIds: new Set(hubs.map((h) => h.data.id)) };
  }

  export async function renderGraph(): Promise<void> {
    if (!graphEl) return;

    const referencedIds = new Set<string>();
    relationships.forEach((r) => {
      const f = (r as any).source_id ?? r.from_entity;
      const t = (r as any).target_id ?? r.to_entity;
      if (f) referencedIds.add(f);
      if (t) referencedIds.add(t);
    });
    const nodeMap = new Map(entities.map((e) => [e.id, e]));
    const placeholderIds = [...referencedIds].filter((id) => !nodeMap.has(id));
    await resolveQidLabels(placeholderIds);

    cy?.destroy();

    const nodes: any[] = entities.map((e) => ({
      data: {
        id: e.id,
        label: e.name,
        type: e.entity_type ?? e.type ?? 'unknown',
        description: e.description ?? e.properties?.description ?? '',
        country: e.country ?? e.properties?.country ?? '',
        source: e.properties?.source ?? '',
      },
    }));

    placeholderIds.forEach((id) => {
      let label = qidLabelCache.get(id);
      if (!label) label = stripSourcePrefix(id);
      label = (label || id).trim() || id;
      nodes.push({
        data: {
          id,
          label,
          type: 'unknown',
          description: '',
          country: '',
          source: '',
          placeholder: 'true',
        },
      });
    });

    const edges = relationships
      .map((r) => {
        const from = (r as any).source_id ?? r.from_entity;
        const to = (r as any).target_id ?? r.to_entity;
        const relType = (r as any).relation_type ?? r.relationship_type ?? (r as any).type ?? '';
        const year = extractYear(r.properties);
        return { from, to, relType, rawId: r.id, year };
      })
      .filter((e) => e.from && e.to)
      .filter((e) => e.year === null || e.year <= maxYear)
      .map((e) => ({
        data: {
          id: e.rawId,
          source: e.from,
          target: e.to,
          label: e.relType,
          relType: e.relType,
          year: e.year,
        },
      }));

    const stubIds = new Set(
      nodes
        .filter((n) => n.data.placeholder === 'true' || n.data.type === 'stub')
        .map((n) => n.data.id)
    );

    const WEAK_REL_TYPES = new Set(['related', 'related_to', 'related topic', 'mentions']);
    const filteredEdges = hideWeakLinks
      ? edges.filter((e) => {
          const isWeak = WEAK_REL_TYPES.has((e.data.relType ?? '').toLowerCase());
          const touchesStub = stubIds.has(e.data.target) || stubIds.has(e.data.source);
          return !(isWeak && touchesStub);
        })
      : edges;

    const deg = new Map<string, number>();
    filteredEdges.forEach((e) => {
      deg.set(e.data.source, (deg.get(e.data.source) ?? 0) + 1);
      deg.set(e.data.target, (deg.get(e.data.target) ?? 0) + 1);
    });

    const maxDegree = Math.max(...deg.values(), 0);

    const visibleNodes = showIsolated
      ? nodes
      : nodes.filter((n) => (deg.get(n.data.id) ?? 0) > 0 || n.data.id === focusHubId);

    const { color: clusterColor, hubIds } = assignClusterColors(visibleNodes, filteredEdges, deg, maxDegree);

    visibleNodes.forEach((n) => {
      const d = deg.get(n.data.id) ?? 0;
      n.data.degree = d;
      const ratio = maxDegree > 0 ? d / maxDegree : 0;
      n.data.size = Math.min(24 + 46 * ratio * ratio, 70);
      n.data.color = clusterColor.get(n.data.id);
      n.data.hot = hubIds.has(n.data.id) ? 'true' : 'false';
    });

    nodeCount = visibleNodes.length;
    edgeCount = filteredEdges.length;

    const showEdgeLabel = edgeCount < 30;

    const layoutOpts: any = {
      name: graphLayout,
      animate: false,
      padding: 30,
      ...(graphLayout === 'concentric' && {
        minNodeSpacing: 30,
        avoidOverlap: true,
        concentric: (node: any) => node.data('degree') ?? 0,
        levelWidth: () => 1,
      }),
      ...(graphLayout === 'breadthfirst' && {
        directed: true,
        spacingFactor: 1.2,
        avoidOverlap: true,
      }),
    };

    cy = cytoscape({
      container: graphEl,
      elements: [...visibleNodes, ...filteredEdges],
      style: [
        {
          selector: 'node',
          style: {
            label: 'data(label)',
            'background-color': (ele: any) => ele.data('color') ?? colorForType(ele.data('type')),
            color: 'rgba(255,255,255,0.9)',
            'font-size': 10,
            'font-weight': 500,
            'text-valign': 'bottom',
            'text-margin-y': 4,
            'text-wrap': 'ellipsis',
            'text-max-width': '80px',
            width: 'data(size)',
            height: 'data(size)',
            'border-width': 0,
            'border-color': 'rgba(255,255,255,0.25)',
          },
        },
        {
          selector: 'node[placeholder = "true"]',
          style: {
            'background-color': '#4a5568',
            'border-width': 2,
            'border-style': 'dashed',
            'border-color': '#718096',
            opacity: 0.7,
          },
        },
        {
          selector: 'node[hot = "true"]',
          style: {
            'border-width': 3,
            'border-color': '#2dd4bf',
            'border-opacity': 0.9,
          },
        },
        {
          selector: 'node:selected',
          style: {
            'border-width': 3,
            'border-color': 'rgba(255,255,255,0.25)',
            'overlay-color': '#2dd4bf',
            'overlay-opacity': 0.15,
          },
        },
        {
          selector: 'node:active',
          style: {
            'overlay-color': '#2dd4bf',
            'overlay-opacity': 0.25,
          },
        },
        {
          selector: 'edge',
          style: {
            label: showEdgeLabel ? 'data(label)' : '',
            'line-color': (ele: any) => colorForEdge(ele.data('relType')),
            'target-arrow-color': (ele: any) => colorForEdge(ele.data('relType')),
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            'font-size': 8,
            color: 'rgba(255,255,255,0.5)',
            width: (ele: any) => (ele.data('relType') === 'parent_of' ? 2.5 : 1.5),
            opacity: 0.85,
          },
        },
        {
          selector: 'edge:selected',
          style: {
            width: 4,
            opacity: 1,
          },
        },
      ],
      layout: layoutOpts,
    });

    cy.on('tap', 'node', (evt: any) => {
      const d = evt.target.data();
      selectedNode = d;
      dispatch('nodeSelected', d);
      cy!.elements().removeClass('dim');
      const neighborhood = evt.target.closedNeighborhood();
      cy!.elements().not(neighborhood).addClass('dim');
    });

    cy.on('tap', (evt: any) => {
      if (evt.target === cy) {
        selectedNode = null;
        dispatch('nodeCleared');
        cy!.elements().removeClass('dim');
      }
    });

    cy.style()
      .selector('.dim')
      .style({ opacity: 0.25 })
      .update();
  }

  export function focusHub(hubId: string): void {
    if (!cy) return;
    const node = cy.getElementById(hubId);
    if (node && node.length) {
      selectedNode = node.data();
      dispatch('nodeSelected', selectedNode!);
      cy.animate({
        center: { eles: node },
        zoom: { level: 1.2, position: node.position() },
        duration: 300,
      });
      node.select();
      const neighborhood = node.closedNeighborhood();
      cy.elements().removeClass('dim');
      cy.elements().not(neighborhood).addClass('dim');
    }
  }

  onMount(() => {
    // Initial render happens when parent passes entities/relationships and calls renderGraph()
  });

  onDestroy(() => {
    cy?.destroy();
  });

  function handleExploreFromNode() {
    if (selectedNode) dispatch('exploreFromNode', { query: selectedNode.label });
  }

  function handleEnrichFromNode() {
    if (selectedNode) dispatch('enrichFromNode', { query: selectedNode.label });
  }
</script>

<div class="view-toolbar">
  <span class="stats">
    {nodeCount} nœuds · {edgeCount} arêtes{scopeLabel ? ` · ${scopeLabel}` : ''}
    {#if truncated}<span class="truncated" title="Le backend a plafonné le nombre de relations renvoyées">· tronqué</span>{/if}
  </span>
  <button class="secondary small" on:click={() => { showIsolated = !showIsolated; renderGraph(); }}>
    {showIsolated ? '👁️ Masquer orphelins' : '👁️‍🗨️ Tout afficher'}
  </button>
  <button class="secondary small" on:click={() => { hideWeakLinks = !hideWeakLinks; renderGraph(); }}>
    {hideWeakLinks ? '🔗 Liens forts' : '🔗 Tous liens'}
  </button>
  <span class="year-slider" title="Afficher uniquement les données jusqu'à {maxYear}">
    <span class="year-label">📅 {maxYear}</span>
    <input
      type="range"
      min={MIN_YEAR}
      max={new Date().getFullYear()}
      bind:value={maxYear}
      on:input={renderGraph}
    />
  </span>
  <select bind:value={graphLayout} on:change={renderGraph}>
    <option value="cose">cose</option>
    <option value="breadthfirst">hiérarchie</option>
    <option value="concentric">concentrique</option>
    <option value="circle">cercle</option>
  </select>
  <button class="secondary small" on:click={() => cy?.fit(undefined, 30)}>⟲ Recentrer</button>
</div>

<div class="graph-container">
  <div class="graph-wrap" bind:this={graphEl}></div>
  {#if selectedNode}
    <aside class="node-panel">
      <div class="panel-head">
        <h3>{selectedNode.label ?? selectedNode.id ?? 'Entité'}</h3>
        <button class="ghost icon" aria-label="Fermer" on:click={() => { selectedNode = null; dispatch('nodeCleared'); cy?.elements().removeClass('dim'); }}>×</button>
      </div>
      <div class="panel-body">
        <div class="kv"><span class="k">Type</span><span class="v badge">{selectedNode.type}</span></div>
        {#if selectedNode.country}<div class="kv"><span class="k">Pays</span><span class="v">{selectedNode.country}</span></div>{/if}
        {#if selectedNode.source}<div class="kv"><span class="k">Source</span><span class="v src-badge">{selectedNode.source}</span></div>{/if}
        <div class="kv"><span class="k">Connexions</span><span class="v">{selectedNode.degree}</span></div>
        {#if selectedNode.description}
          <p class="desc">{selectedNode.description.slice(0, 280)}{selectedNode.description.length > 280 ? '…' : ''}</p>
        {/if}
        <div class="panel-actions">
          <button class="secondary small" on:click={handleExploreFromNode}>
            🔎 Explorer depuis ce nœud
          </button>
          {#if selectedNode.placeholder === 'true' || selectedNode.type === 'stub'}
            <button class="secondary small enrich" on:click={handleEnrichFromNode}>
              ⬆️ Enrichir ce stub
            </button>
          {/if}
        </div>
      </div>
    </aside>
  {/if}
</div>

<style>


  .year-slider {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .year-label {
    color: #94a3b8;
    font-size: 0.85rem;
    min-width: 60px;
  }

  .year-slider input[type='range'] {
    width: 130px;
  }


  .graph-container {
    display: flex;
    gap: 1rem;
    align-items: flex-start;
  }

  .truncated {
    color: var(--warn);
  }

  .graph-wrap {
    flex: 1;
    height: calc(100vh - 230px);
    min-height: 400px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    backdrop-filter: blur(20px);
  }

  .node-panel {
    width: 320px;
    background: rgba(255, 255, 255, 0.04);
    backdrop-filter: blur(20px) saturate(180%);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    padding: 1.25rem;
    max-height: 70vh;
    overflow-y: auto;
  }

  .panel-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  .panel-head h3 {
    font-size: 1rem;
    color: #e2e8f0;
    margin: 0;
    flex: 1;
  }



  .panel-body {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .kv {
    display: flex;
    justify-content: space-between;
    font-size: 0.85rem;
  }

  .kv .k {
    color: #94a3b8;
  }

  .kv .v {
    color: #e2e8f0;
    text-align: right;
    max-width: 60%;
  }

  .desc {
    color: #94a3b8;
    font-size: 0.85rem;
    line-height: 1.5;
    margin: 0.5rem 0;
  }

  .panel-actions {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 1rem;
  }

</style>
