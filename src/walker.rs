use slotmap::SecondaryMap;

use crate::{
    Graph, INVALID_STATE, Node, NodeId, OutputPortId,
    analyzer::GraphAnalyzer,
    reference::{
        InputPortReference, NodeInputIdentifier, NodeOutputIdentifier, OutputPortReference,
    },
};

pub type OutputCache<T> = SecondaryMap<OutputPortId, T>;

pub struct GraphWalkContext<'a, 'b, N: Node> {
    graph: &'a Graph<N>,
    output_cache: &'b mut OutputCache<N::DataValue>,
    node: NodeId,
}

impl<'a, 'b, N: Node> GraphWalkContext<'a, 'b, N> {
    /// Get the computed output of an input port
    pub fn get<'c>(&self, input: impl NodeInputIdentifier<'c>) -> N::DataValue {
        let input = input.combine(self.node);

        self.graph
            .get_incoming_connections(input)
            .filter_map(|port| self.output_cache.get(port))
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

    pub fn get_all<'c>(
        &self,
        input: impl NodeInputIdentifier<'c>,
    ) -> impl Iterator<Item = N::DataValue> + '_ {
        let input = input.combine(self.node);

        self.graph
            .get_incoming_connections(input)
            .filter_map(|port| self.output_cache.get(port))
            .cloned()
    }

    /// Set the value of an output port
    pub fn set<'c>(
        &mut self,
        output: impl NodeOutputIdentifier<'c>,
        value: impl Into<N::DataValue>,
    ) {
        let value: N::DataValue = value.into();
        let output = output.combine(self.node);

        self.output_cache.insert(
            output
                .resolve(self.graph)
                .expect("Output port does not exist"),
            value,
        );
    }

    pub fn can_get(&self, input: impl NodeInputIdentifier<'a>) -> bool {
        input.combine(self.node).resolve(self.graph).is_some()
    }

    pub fn can_set(&self, input: impl NodeOutputIdentifier<'a>) -> bool {
        input.combine(self.node).resolve(self.graph).is_some()
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

    pub fn from_path(
        graph: &'a Graph<N>,
        path: Vec<NodeId>,
        cache: Option<SecondaryMap<OutputPortId, N::DataValue>>,
    ) -> Self {
        Self {
            graph,
            path,
            output_cache: cache
                .unwrap_or_else(|| SecondaryMap::with_capacity(graph.node_data.len())),
        }
    }

    pub fn walk<F: for<'b> Fn(&mut N, &mut GraphWalkContext<'a, 'b, N>)>(&mut self, callback: F) {
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

    pub fn release_cache(self) -> SecondaryMap<OutputPortId, N::DataValue> {
        self.output_cache
    }
}
