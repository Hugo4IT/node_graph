use slotmap::SecondaryMap;

use crate::{
    Graph, INVALID_STATE, Node, NodeId, OutputPortId,
    analyzer::GraphAnalyzer,
    reference::{NodeInputIdentifier, NodeOutputIdentifier, OutputPortReference},
};

pub struct GraphWalkContext<'a, 'b, N: Node> {
    graph: &'a Graph<N>,
    output_cache: &'b mut SecondaryMap<OutputPortId, N::DataValue>,
    node: NodeId,
}

impl<'a, 'b, N: Node> GraphWalkContext<'a, 'b, N> {
    /// Get the computed output of an input port
    pub fn get<'c>(&self, input: impl NodeInputIdentifier<'c>) -> N::DataValue {
        let input = input.combine(self.node);

        self.graph
            .get_incoming_connections(input)
            .map(|port| {
                self.output_cache
                    .get(port)
                    .expect("Missing output value of dependency node")
            })
            .cloned()
            .next()
            .unwrap_or_else(|| {
                self.graph
                    .get_input_port_info(input)
                    .expect(INVALID_STATE)
                    .default
                    .clone()
                    .expect("No default value present for disconnected port")
            })
    }

    /// Set the value of an output port
    pub fn set<'c>(&mut self, output: impl NodeOutputIdentifier<'c>, value: N::DataValue) {
        let output = output.combine(self.node);

        if self
            .output_cache
            .insert(output.resolve(self.graph), value)
            .is_some()
        {
            eprintln!("WARN: An output value was set twice (or cache was not cleared)");
        }
    }
}

#[derive(Debug)]
pub struct GraphWalker<'a, N: Node> {
    graph: &'a Graph<N>,
    path: Vec<NodeId>,
    output_cache: SecondaryMap<OutputPortId, N::DataValue>,
}

impl<'a, N: Node> GraphWalker<'a, N> {
    /// If `exit_nodes` is left as `None`, exit nodes will automatically be
    /// calculated
    pub fn new(graph: &'a Graph<N>, exit_nodes: Option<&[NodeId]>) -> Self {
        Self {
            graph,
            path: match exit_nodes {
                Some(exit_nodes) => GraphAnalyzer::new(graph).generate_execution_path(exit_nodes),
                None => GraphAnalyzer::new(graph).generate_complete_execution_path(),
            },
            output_cache: SecondaryMap::with_capacity(graph.node_data.len()),
        }
    }

    pub fn from_path(graph: &'a Graph<N>, path: Vec<NodeId>) -> Self {
        Self {
            graph,
            path,
            output_cache: SecondaryMap::with_capacity(graph.node_data.len()),
        }
    }

    pub fn walk<F: for<'b> Fn(&mut N, &mut GraphWalkContext<'a, 'b, N>)>(&mut self, callback: F) {
        self.output_cache.clear();

        for &id in self.path.iter() {
            let mut node = self.graph.get_node_mut(id).expect(INVALID_STATE);
            let mut context = GraphWalkContext {
                graph: self.graph,
                output_cache: &mut self.output_cache,
                node: id,
            };

            callback(&mut node, &mut context);
        }
    }

    pub fn graph(&'a self) -> &'a Graph<N> {
        self.graph
    }

    pub fn path(&self) -> &[NodeId] {
        &self.path
    }

    pub fn get<'b>(&'b mut self, node: NodeId) -> GraphWalkContext<'a, 'b, N> {
        GraphWalkContext {
            graph: self.graph,
            output_cache: &mut self.output_cache,
            node,
        }
    }
}
