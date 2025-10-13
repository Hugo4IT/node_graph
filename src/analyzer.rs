use slotmap::SecondaryMap;

use crate::{Graph, INVALID_STATE, Node, NodeId};

/// This structure is guaranteed to contain the id of each node in the analyzed
/// graph exactly once.
#[derive(Debug, Clone, Default)]
pub struct CatagorizedNodes {
    /// The ids of all nodes that have no connections at all
    pub loose: Vec<NodeId>,
    /// The ids of all nodes that exclusively have output connections
    pub entry: Vec<NodeId>,
    /// The ids of all nodes that exclusively have input connections
    pub exit: Vec<NodeId>,
    /// The ids of all nodes that have both input *and* output connections
    pub net: Vec<NodeId>,
}

#[derive(Debug)]
pub struct GraphAnalyzer<'a, N: Node> {
    graph: &'a Graph<N>,
}

impl<'a, N: Node> GraphAnalyzer<'a, N> {
    pub fn new(graph: &'a Graph<N>) -> Self {
        Self { graph }
    }

    pub fn catagorize_nodes(&self) -> CatagorizedNodes {
        let count = self.graph.node_data.len();

        let mut nodes = CatagorizedNodes {
            loose: Vec::with_capacity(4),
            entry: Vec::with_capacity(count >> 2),
            exit: Vec::with_capacity(count >> 2),
            net: Vec::with_capacity(count),
        };

        for (id, node) in self.graph.node_data.iter() {
            let has_incoming_connections = node.inputs.iter().any(|(_, port)| {
                self.graph
                    .get_input_port_info(*port)
                    .expect(INVALID_STATE)
                    .incoming_connections
                    .len()
                    > 0
            });

            let has_outgoing_connections = node.outputs.iter().any(|(_, port)| {
                self.graph
                    .get_output_port_info(*port)
                    .expect(INVALID_STATE)
                    .outgoing_connections
                    .len()
                    > 0
            });

            match (has_incoming_connections, has_outgoing_connections) {
                (false, false) => nodes.loose.push(id),
                (false, true) => nodes.entry.push(id),
                (true, false) => nodes.exit.push(id),
                (true, true) => nodes.net.push(id),
            }
        }

        nodes
    }

    /// Returns all (non-loose) node ids in the order that ensures dependencies
    /// are always processed before dependants
    pub fn generate_execution_path(&self, exit_nodes: &[NodeId]) -> Vec<NodeId> {
        let mut buffer = SecondaryMap::<NodeId, usize>::with_capacity(self.graph.node_data.len());

        for &exit in exit_nodes {
            let mut stack = Vec::new();
            stack.push(exit);

            let mut priority = 0;

            while let Some(top) = stack.pop() {
                let previous_priority = buffer.get(top).copied().unwrap_or(0);
                buffer.insert(top, previous_priority.max(priority));

                stack.extend(self.graph.get_direct_dependencies(top));

                priority += 1;
            }
        }

        let mut buffer = buffer.into_iter().collect::<Vec<(NodeId, usize)>>();
        buffer.sort_by_key(|(_, priority)| *priority);

        buffer.iter().rev().map(|(id, _)| *id).collect()
    }

    /// Returns all (non-loose) node ids in the order that ensures dependencies
    /// are always processed before dependants
    pub fn generate_complete_execution_path(&self) -> Vec<NodeId> {
        self.generate_execution_path(&self.catagorize_nodes().exit)
    }
}
