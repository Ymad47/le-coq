#!/usr/bin/env python3
"""Analyse structure Le Coq avec graphify"""

from pathlib import Path
from graphify.extract import extract
from graphify.build import build_from_json as build_graph
from graphify.export import to_json, to_html
from graphify.analyze import god_nodes, surprising_connections
from graphify.cluster import cluster, score_all
import json

def main():
    PROJECT_ROOT = Path("/home/yaya/le-coq")
    DOCS_DIR = PROJECT_ROOT / "docs"
    DOCS_DIR.mkdir(exist_ok=True)

    print("→ Collecte des fichiers...")
    all_files = []
    for pattern in ["*.py", "*.rs", "*.ts", "*.tsx", "*.js"]:
        for f in PROJECT_ROOT.rglob(pattern):
            if any(x in str(f) for x in ["venv", "target", ".git", "node_modules", "zig-cache"]):
                continue
            all_files.append(f)

    print(f"✓ {len(all_files)} fichiers trouvés")

    print("\n→ Extraction AST...")
    graph_data = extract(
        paths=all_files,
        cache_root=PROJECT_ROOT / "graphify-out" / "cache",
        parallel=False
    )

    print(f"✓ {len(graph_data.get('nodes', []))} noeuds, {len(graph_data.get('edges', []))} arêtes")

    json_path = DOCS_DIR / "code_graph.json"
    with open(json_path, 'w') as f:
        json.dump(graph_data, f, indent=2)
    print(f"✓ Graphe brut: {json_path}")

    print("\n→ Construction graphe...")
    G = build_graph(graph_data)

    print("\n=== STATS DU PROJET ===")
    print(f"Noeuds: {G.number_of_nodes()}")
    print(f"Arêtes: {G.number_of_edges()}")

    print("\n=== GOD NODES ===")
    gods = god_nodes(G, top_n=10)
    for g in gods:
        print(f"  {g.get('node', g.get('id', 'N/A'))}: score={g.get('score', 0):.2f}")

    print("\n=== CLUSTERS ===")
    communities = cluster(G)
    for i, members in list(communities.items())[:5]:
        print(f"  Cluster {i}: {len(members)} noeuds")
        for f in members[:3]:
            print(f"    - {f}")

    cohesion = score_all(G, communities)
    print(f"\nCohésion globale: {cohesion.get('overall_cohesion', 0):.2f}")

    print("\n=== CONNEXIONS SURPRENANTES ===")
    surprises = surprising_connections(G)
    for s in list(surprises)[:5]:
        print(f"  {s}")

    print("\n→ Génération HTML...")
    html_path = DOCS_DIR / "architecture_viz.html"
    to_html(
        G=G,
        communities=communities,
        output_path=str(html_path),
        community_labels={i: f"Cluster {i+1}" for i in communities.keys()}
    )
    print(f"✓ Visualisation: {html_path}")
    
    json_viz_path = DOCS_DIR / "graph_for_viz.json"
    to_json(
        G=G,
        communities=communities,
        output_path=str(json_viz_path),
        community_labels={i: f"Cluster {i+1}" for i in communities.keys()}
    )
    print(f"✓ Graphe enrichi: {json_viz_path}")
    
    print("\n=== ANALYSE TERMINÉE ===")

if __name__ == "__main__":
    main()
