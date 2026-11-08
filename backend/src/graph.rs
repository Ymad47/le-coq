//! Graph algorithms over the entity/relationship graph.
//!
//! Pure, in-memory and synchronous: the API layer loads edges from PostgreSQL,
//! builds a [`Graph`] and runs these on the blocking thread pool.
//!
//! - Centrality: directed PageRank (incoming links from important nodes) plus
//!   in/out degree. Unweighted: `confidence` is often NULL ("unknown").
//! - Shortest path: BFS ignoring edge direction (a link is a link for an
//!   analyst), unweighted, bounded depth.
//! - Communities: Louvain modularity optimization, deterministic (nodes are
//!   visited in id order).

use std::collections::{HashMap, HashSet, VecDeque};

/// One relationship as loaded from the database.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relationship_type: String,
}

/// Relationship types too generic to carry analytical signal (Wikipedia
/// "see also" links, mentions). Excluded when `typed_only` is requested.
pub const GENERIC_REL_TYPES: &[&str] = &["related", "related_to", "related topic", "mentions"];

/// Directed multigraph with dense node indices. Node `i` is `ids[i]`; ids are
/// sorted so every algorithm below is deterministic.
#[derive(Debug, Default)]
pub struct Graph {
    ids: Vec<String>,
    index: HashMap<String, usize>,
    edges: Vec<Edge>,
    /// Distinct successors per node (self-loops and parallel edges removed).
    out: Vec<Vec<usize>>,
    /// Undirected neighbors: (neighbor, edge index). Parallel edges kept so a
    /// path can report the relationship it went through.
    undirected: Vec<Vec<(usize, usize)>>,
}

impl Graph {
    pub fn new(edges: Vec<Edge>) -> Self {
        let mut ids: Vec<String> = edges
            .iter()
            .flat_map(|e| [e.from.clone(), e.to.clone()])
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        ids.sort();
        let index: HashMap<String, usize> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();

        let n = ids.len();
        let mut out_sets: Vec<HashSet<usize>> = vec![HashSet::new(); n];
        let mut undirected: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
        for (k, e) in edges.iter().enumerate() {
            let (a, b) = (index[&e.from], index[&e.to]);
            if a == b {
                continue;
            }
            out_sets[a].insert(b);
            undirected[a].push((b, k));
            undirected[b].push((a, k));
        }
        let out = out_sets
            .into_iter()
            .map(|s| {
                let mut v: Vec<usize> = s.into_iter().collect();
                v.sort_unstable();
                v
            })
            .collect();

        Self {
            ids,
            index,
            edges,
            out,
            undirected,
        }
    }

    pub fn node_count(&self) -> usize {
        self.ids.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.index.contains_key(id)
    }

    /// Number of distinct neighbors, ignoring direction.
    fn degree(&self, i: usize) -> usize {
        self.undirected[i]
            .iter()
            .map(|&(j, _)| j)
            .collect::<HashSet<_>>()
            .len()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Centrality {
    pub id: String,
    pub pagerank: f64,
    pub in_degree: usize,
    pub out_degree: usize,
}

const DAMPING: f64 = 0.85;
const PAGERANK_MAX_ITER: usize = 100;
const PAGERANK_TOLERANCE: f64 = 1e-9;

/// PageRank for every node, scores summing to 1, sorted by score descending
/// (ties by id). Dangling nodes spread their rank uniformly.
pub fn centrality(g: &Graph) -> Vec<Centrality> {
    let n = g.node_count();
    if n == 0 {
        return Vec::new();
    }
    let nf = n as f64;
    let mut rank = vec![1.0 / nf; n];
    let mut next = vec![0.0; n];

    for _ in 0..PAGERANK_MAX_ITER {
        let dangling: f64 = (0..n)
            .filter(|&i| g.out[i].is_empty())
            .map(|i| rank[i])
            .sum();
        let base = (1.0 - DAMPING) / nf + DAMPING * dangling / nf;
        next.iter_mut().for_each(|x| *x = base);
        for (i, succ) in g.out.iter().enumerate() {
            if succ.is_empty() {
                continue;
            }
            let share = DAMPING * rank[i] / succ.len() as f64;
            for &j in succ {
                next[j] += share;
            }
        }
        let delta: f64 = rank.iter().zip(&next).map(|(a, b)| (a - b).abs()).sum();
        std::mem::swap(&mut rank, &mut next);
        if delta < PAGERANK_TOLERANCE {
            break;
        }
    }

    let mut in_degree = vec![0usize; n];
    for succ in &g.out {
        for &j in succ {
            in_degree[j] += 1;
        }
    }

    let mut result: Vec<Centrality> = (0..n)
        .map(|i| Centrality {
            id: g.ids[i].clone(),
            pagerank: rank[i],
            in_degree: in_degree[i],
            out_degree: g.out[i].len(),
        })
        .collect();
    result.sort_by(|a, b| {
        b.pagerank
            .total_cmp(&a.pagerank)
            .then_with(|| a.id.cmp(&b.id))
    });
    result
}

/// A shortest path: `nodes[0]` is the start, and `edges[k]` links
/// `nodes[k]` and `nodes[k + 1]` (in either direction).
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub nodes: Vec<String>,
    pub edges: Vec<Edge>,
}

/// Unweighted shortest path ignoring direction, at most `max_depth` hops.
/// None if an endpoint is missing or no path is short enough.
pub fn shortest_path(g: &Graph, from: &str, to: &str, max_depth: usize) -> Option<Path> {
    let (&s, &t) = (g.index.get(from)?, g.index.get(to)?);
    if s == t {
        return Some(Path {
            nodes: vec![g.ids[s].clone()],
            edges: Vec::new(),
        });
    }

    // parent[v] = (previous node, edge index used to reach v)
    let mut parent: Vec<Option<(usize, usize)>> = vec![None; g.node_count()];
    let mut depth = vec![usize::MAX; g.node_count()];
    depth[s] = 0;
    let mut queue = VecDeque::from([s]);

    while let Some(u) = queue.pop_front() {
        if depth[u] >= max_depth {
            continue;
        }
        for &(v, k) in &g.undirected[u] {
            if depth[v] != usize::MAX {
                continue;
            }
            depth[v] = depth[u] + 1;
            parent[v] = Some((u, k));
            if v == t {
                return Some(rebuild_path(g, &parent, s, t));
            }
            queue.push_back(v);
        }
    }
    None
}

fn rebuild_path(g: &Graph, parent: &[Option<(usize, usize)>], s: usize, t: usize) -> Path {
    let mut nodes = vec![t];
    let mut edges = Vec::new();
    let mut cur = t;
    while cur != s {
        let (prev, k) = parent[cur].expect("BFS parent chain reaches the start");
        edges.push(g.edges[k].clone());
        nodes.push(prev);
        cur = prev;
    }
    nodes.reverse();
    edges.reverse();
    Path {
        nodes: nodes.into_iter().map(|i| g.ids[i].clone()).collect(),
        edges,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Community {
    /// Member ids, most connected first (ties by id).
    pub members: Vec<String>,
}

/// Louvain community detection (modularity optimization, Blondel et al. 2008)
/// on the undirected simple graph: direction, parallel edges and self-loops
/// are ignored. Deterministic: nodes are visited in index (id) order and
/// candidate communities in ascending order.
/// Communities sorted by size descending; filter small ones at the call site.
pub fn communities(g: &Graph) -> Vec<Community> {
    let n = g.node_count();
    if n == 0 {
        return Vec::new();
    }

    // Level-0 graph: one weight-1 edge per distinct unordered pair.
    let mut adj: Vec<Vec<(usize, f64)>> = (0..n)
        .map(|u| {
            let mut ns: Vec<usize> = g.undirected[u].iter().map(|&(v, _)| v).collect();
            ns.sort_unstable();
            ns.dedup();
            ns.into_iter().map(|v| (v, 1.0)).collect()
        })
        .collect();
    let mut self_loops = vec![0.0; n];
    // membership[original node] = node in the current level graph
    let mut membership: Vec<usize> = (0..n).collect();

    loop {
        let (comm, moved) = louvain_local_moves(&adj, &self_loops);
        if !moved {
            break;
        }
        let (next_adj, next_loops, renumber) = louvain_aggregate(&adj, &self_loops, &comm);
        for m in membership.iter_mut() {
            *m = renumber[comm[*m]];
        }
        if next_adj.len() == adj.len() {
            break; // no merge happened: converged
        }
        adj = next_adj;
        self_loops = next_loops;
    }

    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for (u, &c) in membership.iter().enumerate() {
        groups.entry(c).or_default().push(u);
    }
    let degree: Vec<usize> = (0..n).map(|i| g.degree(i)).collect();
    let mut result: Vec<Community> = groups
        .into_values()
        .map(|mut members| {
            members.sort_by(|&a, &b| degree[b].cmp(&degree[a]).then_with(|| a.cmp(&b)));
            Community {
                members: members.into_iter().map(|i| g.ids[i].clone()).collect(),
            }
        })
        .collect();
    result.sort_by(|a, b| {
        b.members
            .len()
            .cmp(&a.members.len())
            .then_with(|| a.members[0].cmp(&b.members[0]))
    });
    result
}

const LOUVAIN_MAX_PASSES: usize = 100;
const LOUVAIN_EPSILON: f64 = 1e-12;

/// Phase 1: move each node to the neighboring community with the best
/// modularity gain until no move improves it. Returns the community of each
/// node and whether anything moved.
fn louvain_local_moves(adj: &[Vec<(usize, f64)>], self_loops: &[f64]) -> (Vec<usize>, bool) {
    let n = adj.len();
    // k[u]: weighted degree, self-loops counted twice; sum(k) = 2m.
    let k: Vec<f64> = (0..n)
        .map(|u| adj[u].iter().map(|&(_, w)| w).sum::<f64>() + 2.0 * self_loops[u])
        .collect();
    let two_m: f64 = k.iter().sum();
    let mut comm: Vec<usize> = (0..n).collect();
    if two_m == 0.0 {
        return (comm, false);
    }
    let mut tot = k.clone(); // total degree per community
    let mut moved_any = false;
    let mut links: HashMap<usize, f64> = HashMap::new();

    for _ in 0..LOUVAIN_MAX_PASSES {
        let mut moved = false;
        for u in 0..n {
            links.clear();
            for &(v, w) in &adj[u] {
                *links.entry(comm[v]).or_insert(0.0) += w;
            }
            let own = comm[u];
            tot[own] -= k[u];
            // Gain of joining c (up to a constant factor): k_u,in(c) - tot(c)·k_u / 2m
            let gain = |c: usize, w: f64| w - tot[c] * k[u] / two_m;
            let mut best = own;
            let mut best_gain = gain(own, links.get(&own).copied().unwrap_or(0.0));
            let mut candidates: Vec<(usize, f64)> = links.iter().map(|(&c, &w)| (c, w)).collect();
            candidates.sort_by_key(|&(c, _)| c);
            for (c, w) in candidates {
                let g = gain(c, w);
                if g > best_gain + LOUVAIN_EPSILON {
                    best = c;
                    best_gain = g;
                }
            }
            tot[best] += k[u];
            if best != own {
                comm[u] = best;
                moved = true;
                moved_any = true;
            }
        }
        if !moved {
            break;
        }
    }
    (comm, moved_any)
}

/// Phase 2: collapse each community into one node. Returns the new adjacency,
/// self-loop weights and the old-community -> new-node mapping.
#[allow(clippy::type_complexity)]
fn louvain_aggregate(
    adj: &[Vec<(usize, f64)>],
    self_loops: &[f64],
    comm: &[usize],
) -> (Vec<Vec<(usize, f64)>>, Vec<f64>, Vec<usize>) {
    // Dense renumbering in order of first appearance (deterministic).
    let mut renumber = vec![usize::MAX; adj.len()];
    let mut count = 0;
    for &c in comm {
        if renumber[c] == usize::MAX {
            renumber[c] = count;
            count += 1;
        }
    }
    let mut weights: Vec<HashMap<usize, f64>> = vec![HashMap::new(); count];
    let mut loops = vec![0.0; count];
    for (u, neighbors) in adj.iter().enumerate() {
        let cu = renumber[comm[u]];
        loops[cu] += self_loops[u];
        for &(v, w) in neighbors {
            let cv = renumber[comm[v]];
            if cu == cv {
                loops[cu] += w / 2.0; // each internal edge is seen from both ends
            } else {
                *weights[cu].entry(cv).or_insert(0.0) += w;
            }
        }
    }
    let next_adj = weights
        .into_iter()
        .map(|m| {
            let mut v: Vec<(usize, f64)> = m.into_iter().collect();
            v.sort_by_key(|&(c, _)| c);
            v
        })
        .collect();
    (next_adj, loops, renumber)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(from: &str, to: &str) -> Edge {
        Edge {
            from: from.into(),
            to: to.into(),
            relationship_type: "linked".into(),
        }
    }

    fn graph(pairs: &[(&str, &str)]) -> Graph {
        Graph::new(pairs.iter().map(|(a, b)| e(a, b)).collect())
    }

    #[test]
    fn empty_graph() {
        let g = Graph::new(Vec::new());
        assert_eq!(g.node_count(), 0);
        assert!(centrality(&g).is_empty());
        assert!(communities(&g).is_empty());
        assert!(shortest_path(&g, "a", "b", 5).is_none());
    }

    #[test]
    fn pagerank_sums_to_one_and_ranks_the_hub_first() {
        // Star: every spoke points at the hub.
        let g = graph(&[("a", "hub"), ("b", "hub"), ("c", "hub"), ("d", "hub")]);
        let c = centrality(&g);
        let total: f64 = c.iter().map(|x| x.pagerank).sum();
        assert!((total - 1.0).abs() < 1e-6, "sum = {}", total);
        assert_eq!(c[0].id, "hub");
        assert_eq!(c[0].in_degree, 4);
        assert_eq!(c[0].out_degree, 0);
        // Spokes are symmetric: equal scores, ordered by id.
        let spokes: Vec<&str> = c[1..].iter().map(|x| x.id.as_str()).collect();
        assert_eq!(spokes, vec!["a", "b", "c", "d"]);
        assert!((c[1].pagerank - c[4].pagerank).abs() < 1e-12);
    }

    #[test]
    fn pagerank_cycle_is_uniform() {
        let g = graph(&[("a", "b"), ("b", "c"), ("c", "a")]);
        for x in centrality(&g) {
            assert!((x.pagerank - 1.0 / 3.0).abs() < 1e-6);
        }
    }

    #[test]
    fn parallel_edges_and_self_loops_do_not_inflate_rank() {
        let mut edges = vec![e("a", "b"), e("a", "a")];
        edges.push(Edge {
            relationship_type: "other".into(),
            ..e("a", "b")
        });
        let g = Graph::new(edges);
        let c = centrality(&g);
        let a = c.iter().find(|x| x.id == "a").unwrap();
        assert_eq!(a.out_degree, 1);
        assert_eq!(g.edge_count(), 3); // edges are kept, only scoring dedups
    }

    #[test]
    fn path_ignores_direction_and_reports_edges() {
        // a -> b <- c -> d
        let g = graph(&[("a", "b"), ("c", "b"), ("c", "d")]);
        let p = shortest_path(&g, "a", "d", 5).unwrap();
        assert_eq!(p.nodes, vec!["a", "b", "c", "d"]);
        assert_eq!(p.edges.len(), 3);
        assert_eq!(
            (p.edges[1].from.as_str(), p.edges[1].to.as_str()),
            ("c", "b")
        );
    }

    #[test]
    fn path_takes_the_shortcut() {
        let g = graph(&[("a", "b"), ("b", "c"), ("c", "d"), ("a", "d")]);
        let p = shortest_path(&g, "a", "d", 5).unwrap();
        assert_eq!(p.nodes, vec!["a", "d"]);
    }

    #[test]
    fn path_respects_max_depth() {
        let g = graph(&[("a", "b"), ("b", "c"), ("c", "d")]);
        assert!(shortest_path(&g, "a", "d", 2).is_none());
        assert!(shortest_path(&g, "a", "d", 3).is_some());
    }

    #[test]
    fn path_edge_cases() {
        let g = graph(&[("a", "b"), ("c", "d")]);
        assert!(shortest_path(&g, "a", "d", 10).is_none()); // disconnected
        assert!(shortest_path(&g, "a", "zz", 10).is_none()); // unknown
        let p = shortest_path(&g, "a", "a", 10).unwrap();
        assert_eq!(p.nodes, vec!["a"]);
        assert!(p.edges.is_empty());
    }

    #[test]
    fn communities_split_two_cliques_joined_by_a_bridge() {
        let g = graph(&[
            ("a1", "a2"),
            ("a2", "a3"),
            ("a3", "a1"),
            ("a1", "a4"),
            ("a2", "a4"),
            ("a3", "a4"),
            ("b1", "b2"),
            ("b2", "b3"),
            ("b3", "b1"),
            ("b1", "b4"),
            ("b2", "b4"),
            ("b3", "b4"),
            ("a1", "b1"), // bridge
        ]);
        let cs = communities(&g);
        assert_eq!(cs.len(), 2, "{:?}", cs);
        for c in &cs {
            assert_eq!(c.members.len(), 4);
            let prefix = &c.members[0][..1];
            assert!(c.members.iter().all(|m| m.starts_with(prefix)), "{:?}", c);
        }
    }

    #[test]
    fn communities_ring_of_cliques() {
        // 5 cliques of 5 nodes, each linked to the next by one edge.
        let mut edges = Vec::new();
        for c in 0..5 {
            for i in 0..5 {
                for j in (i + 1)..5 {
                    edges.push(e(&format!("c{c}n{i}"), &format!("c{c}n{j}")));
                }
            }
            edges.push(e(&format!("c{c}n0"), &format!("c{}n1", (c + 1) % 5)));
        }
        let cs = communities(&Graph::new(edges));
        assert_eq!(cs.len(), 5, "{:?}", cs);
        for c in &cs {
            let prefix = &c.members[0][..2];
            assert!(c.members.iter().all(|m| m.starts_with(prefix)), "{:?}", c);
        }
    }

    #[test]
    fn communities_are_deterministic() {
        let pairs = [("x", "y"), ("y", "z"), ("p", "q"), ("q", "r"), ("r", "p")];
        assert_eq!(communities(&graph(&pairs)), communities(&graph(&pairs)));
    }

    #[test]
    fn community_members_sorted_by_degree() {
        let g = graph(&[("hub", "a"), ("hub", "b"), ("hub", "c")]);
        let cs = communities(&g);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].members[0], "hub");
    }
}
