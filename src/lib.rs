use petgraph::algo::astar;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{EdgeRef, Bfs, Dfs, Reversed};
use tree_sitter::{Node, Tree};
use std::collections::HashMap;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use bincode::{serialize_into, deserialize_from};
use fixedbitset::FixedBitSet;

pub mod geometry;
use geometry::{GNode,GRange,Edge};

// Import the test module
#[cfg(test)]
mod tests;

// Informational features to dump AST Graph to a DOT file for debugging
#[cfg(feature="informational")]
use petgraph::dot::{Dot, Config};

///
/// Serializable graph -- as PetGraph doesn't provide a direct means
/// to do this.
/// 
#[derive(Serialize,Deserialize)]
pub struct SerializableGraph {
    pub nodes: Vec<GNode>,
    pub edges: Vec<Edge>,
}

///
/// AST Graph
/// 
pub struct ASTGraph {
    pub graph: DiGraph<GNode,()>,
    node_map: HashMap<NodeIndex,usize>,
    source: String,
    title: String, // title of the graph
}

impl ASTGraph {
    pub fn new(source_code: String) -> Self {
        ASTGraph {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            source: source_code,
            title: "".to_string(),
        }
    }

    pub fn title(&self) -> String {
        self.title.clone() 
    }

    pub fn set_title(&mut self, new_title:String) {
        self.title = new_title;
    }

    pub fn name(&self) -> String {
        // naming scheme to come up with unique names for graphs and subgraphs
        let root_index:NodeIndex = 1.into();
        let root_node_id = self.node_map.get(&root_index).unwrap(); // get the root id (this is the tree-sitter id)
        format!("node_{}_graph",root_node_id)
    }

    pub fn node_count(&self) -> usize {
        self.node_map.len()
    }

    pub fn add_node(&mut self, tree_node: Node ) -> NodeIndex {
        let id = tree_node.id();
        let kind_id = tree_node.kind_id();
        let range = GRange::from(tree_node.range());
        let new_node = GNode {
            id: id,
            kind_id: kind_id,
            range: range,
        };
        let node_index = self.graph.add_node( new_node );
        self.node_map.insert(node_index, id );
        node_index
    }

    pub fn get_node(&self, id:NodeIndex) -> Option<usize> {
        self.node_map.get(&id).cloned()
    }

    pub fn get_node_source(&self, id:NodeIndex) -> &str {
        let graph_node = self.graph[id];
        let slice = &self.source[graph_node.range.start_byte..graph_node.range.end_byte];
        slice
    }

    pub fn add_edge(&mut self, parent: NodeIndex, child: NodeIndex) {
        self.graph.add_edge(parent, child, ());
    }

    pub fn build_from_tree(&mut self, tree: &Tree) {
        let root_node = tree.root_node();
        self.traverse_and_build(root_node, None);
    }

    pub fn traverse_and_build(&mut self, tree_node:Node, parent: Option<NodeIndex>) {
        
        let graph_node = self.add_node(tree_node);
        if let Some(parent_node) = parent {
            self.add_edge(parent_node, graph_node);
        }

        for idx in 0..tree_node.child_count() {
            if let Some(child) = tree_node.child(idx) {
                self.traverse_and_build(child, Some(graph_node));
            }
        }
    }

    pub fn extract_subgraphs(&self, kinds_to_split_on:HashSet<u16>) -> Vec<ASTGraph> {
        let mut subgraphs = Vec::new();

        for node in self.graph.node_indices() {
            if kinds_to_split_on.contains( &self.graph[node].kind_id ) {
                let node_range = &self.graph[node].range;
                let split_source = &self.source[node_range.start_byte..node_range.end_byte];
                let subgraph_nodes = self.collect_subgraph_nodes(node);
                let mut subgraph = self.create_subgraph(&subgraph_nodes);
                subgraph.source = split_source.to_string();
                subgraphs.push(subgraph);
            }
        }

        subgraphs
    }

    fn collect_subgraph_nodes(&self, start_node: NodeIndex) -> HashSet<NodeIndex> {
        let mut visited = HashSet::new();
        let mut bfs = Bfs::new(&self.graph, start_node);

        while let Some(node) = bfs.next(&self.graph) {
            visited.insert(node);
        }

        visited
    }

    fn create_subgraph(&self, subgraph_nodes: &HashSet<NodeIndex>) -> ASTGraph {
        let mut digraph = DiGraph::new();
        let mut node_map = HashMap::new();
        let mut original_mapping = HashMap::new();

        for &node in subgraph_nodes {
            let new_node = digraph.add_node(self.graph[node].clone());
            let original_id = self.graph[node].id;
            node_map.insert(node, new_node);
            original_mapping.insert(new_node,original_id );
        }

        for edge in self.graph.edge_references() {
            let source = edge.source();
            let target = edge.target();
            if subgraph_nodes.contains(&source) && subgraph_nodes.contains(&target) {
                digraph.add_edge(node_map[&source], node_map[&target], ());
            }
        }

        let subgraph = ASTGraph {
            graph: digraph,
            node_map: original_mapping,
            source: "".to_string(),
            title: "".to_string()
        };

        subgraph
    }

    fn to_serializable(&self) -> SerializableGraph {
        let nodes = self.graph.node_indices().map(|n| self.graph[n].clone()).collect();
        let edges = self.graph.edge_indices()
            .map(|e| {
                let (source, target) = self.graph.edge_endpoints(e).unwrap();
                Edge {
                    source: source,
                    target: target,
                }
            }).collect();
        SerializableGraph { nodes, edges }
    }

    pub fn from_serializable(serializable_graph: SerializableGraph) -> Self {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Add nodes to the graph and map node IDs to node indices
        for serialized_node in serializable_graph.nodes {
            let node_index = graph.add_node(serialized_node);
            node_map.insert( node_index, serialized_node.id,);
        }

        // Add edges to the graph based on source and target indices
        for serialized_edge in serializable_graph.edges {
            
            graph.add_edge(serialized_edge.source, serialized_edge.target, ()); // Adjust to your Edge type as needed
        }

        // Create a new ASTGraph instance
        ASTGraph {
            graph,
            node_map,
            source: "".to_string(), // Update according to your needs
            title: "".to_string(),
        }
    }
    /// 
    /// Iterators
    ///
    pub fn bfs_iterator(&self, start_node: NodeIndex) -> Bfs<NodeIndex,FixedBitSet> {
        Bfs::new(&self.graph, start_node)
    }

    pub fn dfs_iterator(&self, start_node: NodeIndex) -> Dfs<NodeIndex,FixedBitSet> {
        Dfs::new(&self.graph, start_node) 
    }

    pub fn reversed_dfs_iterator(&self, start_node:NodeIndex) -> Dfs<NodeIndex,FixedBitSet>  {
        let reversed_graph  = Reversed(&self.graph);
        Dfs::new(&reversed_graph, start_node) 
    }


    pub fn path_from_to(&self, start_node: NodeIndex, goal: NodeIndex) -> Option<Vec<NodeIndex>> {
        // use the A* algorithm to get the short path from start -> goal
        let result = astar(
            &self.graph,
            start_node,
            |finish| finish == goal,
            |_| 1, // edge cost (uniform cost here)
            |_| 0, // heuristic cost
        );

        if let Some((_, path)) = result {
            Some(path)
        } else {
            None
        }
    }

    #[cfg(feature="informational")]
    pub fn write_dot_file(&self, filename:String) 
    {
        
        let dot = Dot::with_attr_getters(
            &self.graph,
            &[Config::EdgeNoLabel],
            &|_, er| format!(""),
            &|_, (ni, gn)| format!("label=\"{}\"", gn.kind_id)
        );
        
        // Save the DOT content to a file
        let mut file = File::create(filename.as_str()).expect("Unable to create file");
        write!(file, "{}", dot).expect("Unable to write DOT file"); // bug

    }

}
