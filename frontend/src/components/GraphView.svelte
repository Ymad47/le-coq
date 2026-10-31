<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { createEventDispatcher } from 'svelte';
  import cytoscape from 'cytoscape';
  import type { Entity, Relationship, GraphNode } from '../lib/types';
  import { extractYear, stripSourcePrefix } from '../lib/utils';
  import { vizPalette } from '../lib/tokens';
  import Icon from './Icon.svelte';

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

    // Restrained palette: the most connected nodes in brand blue, the rest neutral
    const palette = vizPalette();
    const ranked = [...visibleNodes]
      .filter((n) => (deg.get(n.data.id) ?? 0) > 0)
      .sort((a, b) => (deg.get(b.data.id) ?? 0) - (deg.get(a.data.id) ?? 0));
    const hubIds = new Set(
      ranked.slice(0, Math.max(3, Math.ceil(visibleNodes.length * 0.08))).map((n) => n.data.id)
    );

    visibleNodes.forEach((n) => {
      const d = deg.get(n.data.id) ?? 0;
      n.data.degree = d;
      const ratio = maxDegree > 0 ? d / maxDegree : 0;
      n.data.size = Math.min(24 + 46 * ratio * ratio, 70);
      n.data.color = stubIds.has(n.data.id) ? palette.nodeStub : hubIds.has(n.data.id) ? palette.nodeHub : palette.node;
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
      // fit() must not blow a small graph up to giant nodes and labels
      maxZoom: 1.6,
      minZoom: 0.15,
      elements: [...visibleNodes, ...filteredEdges],
      style: [
        {
          selector: 'node',
          style: {
            label: 'data(label)',
            'background-color': 'data(color)',
            color: palette.label,
            'font-family': 'Inter Variable, Inter, system-ui, sans-serif',
            'font-size': 10,
            'font-weight': 500,
            'text-valign': 'bottom',
            'text-margin-y': 4,
            'text-wrap': 'ellipsis',
            'text-max-width': '90px',
            width: 'data(size)',
            height: 'data(size)',
            'border-width': 0,
          },
        },
        {
          selector: 'node[placeholder = "true"]',
          style: {
            'border-width': 1.5,
            'border-style': 'dashed',
            'border-color': palette.nodeStub,
            'background-opacity': 0.5,
          },
        },
        {
          selector: 'node[hot = "true"]',
          style: {
            'border-width': 2,
            'border-color': palette.surface,
          },
        },
        {
          selector: 'node:selected',
          style: {
            'border-width': 3,
            'border-color': palette.brandText,
          },
        },
        {
          selector: 'edge',
          style: {
            // Generic links (related_to, mentions) carry no extra information: no label
            label: (ele: any) =>
              showEdgeLabel && !WEAK_REL_TYPES.has((ele.data('relType') ?? '').toLowerCase())
                ? ele.data('label')
                : '',
            'line-color': (ele: any) =>
              WEAK_REL_TYPES.has((ele.data('relType') ?? '').toLowerCase()) ? palette.edge : palette.edgeStrong,
            'target-arrow-color': (ele: any) =>
              WEAK_REL_TYPES.has((ele.data('relType') ?? '').toLowerCase()) ? palette.edge : palette.edgeStrong,
            'target-arrow-shape': 'triangle',
            'arrow-scale': 0.8,
            'curve-style': 'bezier',
            'font-family': 'ui-monospace, SF Mono, monospace',
            'font-size': 8,
            color: palette.text2,
            'text-background-color': palette.surface,
            'text-background-opacity': 0.85,
            'text-background-padding': '2px',
            width: 1.2,
          },
        },
        {
          selector: 'edge:selected',
          style: { width: 3 },
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
    <strong class="num">{nodeCount}</strong> nœuds · <span class="num">{edgeCount}</span> liens{scopeLabel ? ` · ${scopeLabel}` : ''}
    {#if truncated}<span class="truncated" title="Le backend a plafonné le nombre de relations renvoyées">· tronqué</span>{/if}
  </span>
  <button class="secondary small" aria-pressed={showIsolated} on:click={() => { showIsolated = !showIsolated; renderGraph(); }}>
    {showIsolated ? 'Masquer les isolés' : 'Afficher les isolés'}
  </button>
  <button class="secondary small" aria-pressed={!hideWeakLinks} on:click={() => { hideWeakLinks = !hideWeakLinks; renderGraph(); }}>
    {hideWeakLinks ? 'Liens forts uniquement' : 'Tous les liens'}
  </button>
  <label class="year-slider" title="Afficher uniquement les relations datées jusqu'à {maxYear}">
    <span class="year-label">Jusqu'à <span class="mono">{maxYear}</span></span>
    <input
      type="range"
      min={MIN_YEAR}
      max={new Date().getFullYear()}
      bind:value={maxYear}
      on:input={renderGraph}
    />
  </label>
  <select bind:value={graphLayout} on:change={renderGraph} aria-label="Disposition du graphe">
    <option value="cose">Force</option>
    <option value="breadthfirst">Hiérarchie</option>
    <option value="concentric">Concentrique</option>
    <option value="circle">Cercle</option>
  </select>
  <button class="ghost icon" title="Recentrer" aria-label="Recentrer le graphe" on:click={() => cy?.fit(undefined, 30)}>
    <Icon name="expand" size={16} />
  </button>
</div>

<div class="graph-container">
  <div class="graph-wrap" bind:this={graphEl} role="img" aria-label="Graphe des relations entre entités"></div>
  {#if selectedNode}
    <aside class="node-panel panel" aria-label="Nœud sélectionné">
      <div class="panel-head">
        <h3>{selectedNode.label ?? selectedNode.id ?? 'Entité'}</h3>
        <button class="ghost icon" aria-label="Fermer" on:click={() => { selectedNode = null; dispatch('nodeCleared'); cy?.elements().removeClass('dim'); }}>
          <Icon name="close" size={16} />
        </button>
      </div>
      <dl class="kv-list">
        <div class="kv"><dt>Type</dt><dd><span class="badge">{selectedNode.type}</span></dd></div>
        {#if selectedNode.country}<div class="kv"><dt>Pays</dt><dd>{selectedNode.country}</dd></div>{/if}
        {#if selectedNode.source}<div class="kv"><dt>Source</dt><dd class="mono">{selectedNode.source}</dd></div>{/if}
        <div class="kv"><dt>Connexions</dt><dd class="mono">{selectedNode.degree}</dd></div>
        <div class="kv"><dt>Identifiant</dt><dd class="mono id">{selectedNode.id}</dd></div>
      </dl>
      {#if selectedNode.description}
        <p class="desc">{selectedNode.description.slice(0, 280)}{selectedNode.description.length > 280 ? '…' : ''}</p>
      {/if}
      <div class="panel-actions">
        <button class="secondary small" on:click={handleExploreFromNode}>
          <Icon name="search" size={14} /> Explorer depuis ce nœud
        </button>
        {#if selectedNode.placeholder === 'true' || selectedNode.type === 'stub'}
          <button class="secondary small" on:click={handleEnrichFromNode}>
            <Icon name="globe" size={14} /> Enrichir ce nœud en ligne
          </button>
        {/if}
      </div>
    </aside>
  {/if}
</div>

<style>
  .year-slider {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: var(--fs-xs);
    color: var(--text-2);
  }

  .year-slider input[type='range'] {
    width: 120px;
  }

  .truncated {
    color: var(--warning);
    font-weight: 600;
  }

  .graph-container {
    display: flex;
    gap: 0.75rem;
    align-items: flex-start;
  }

  .graph-wrap {
    flex: 1;
    min-width: 0;
    height: calc(100vh - var(--topbar-h) - 140px);
    min-height: 420px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
  }

  .node-panel {
    width: 300px;
    flex-shrink: 0;
    padding: 1rem;
    max-height: calc(100vh - var(--topbar-h) - 140px);
    overflow-y: auto;
  }

  .panel-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .panel-head h3 {
    margin: 0;
    font-size: var(--fs-md);
    font-weight: 650;
  }

  .kv-list {
    margin: 0;
  }

  .kv {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.4rem 0;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-sm);
  }

  .kv dt {
    color: var(--text-2);
  }

  .kv dd {
    margin: 0;
    text-align: right;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .desc {
    color: var(--text-2);
    font-size: var(--fs-sm);
    line-height: 1.55;
    margin: 0.75rem 0 0;
  }

  .panel-actions {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  .panel-actions button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
  }

  @media (max-width: 820px) {
    .graph-container {
      flex-direction: column;
    }

    .graph-wrap {
      width: 100%;
      height: 60vh;
      min-height: 320px;
    }

    .node-panel {
      width: 100%;
      max-height: none;
    }
  }
</style>
