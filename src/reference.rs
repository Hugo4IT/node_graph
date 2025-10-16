use crate::{Graph, InputPortId, Node, NodeId, OutputPortId};

pub trait NodeInputIdentifier<'a> {
    type Reference: InputPortReference;

    fn combine(self, node_id: NodeId) -> Self::Reference;
}

pub trait NodeOutputIdentifier<'a> {
    type Reference: OutputPortReference;

    fn combine(self, node_id: NodeId) -> Self::Reference;
}

impl<'a> NodeInputIdentifier<'a> for usize {
    type Reference = NodeInputIndexReference;

    fn combine(self, node_id: NodeId) -> Self::Reference {
        NodeInputIndexReference(node_id, self)
    }
}

impl<'a> NodeOutputIdentifier<'a> for usize {
    type Reference = NodeOutputIndexReference;

    fn combine(self, node_id: NodeId) -> Self::Reference {
        NodeOutputIndexReference(node_id, self)
    }
}

impl<'a> NodeInputIdentifier<'a> for &'a str {
    type Reference = NodeInputNameReference<'a>;

    fn combine(self, node_id: NodeId) -> Self::Reference {
        NodeInputNameReference(node_id, self)
    }
}

impl<'a> NodeOutputIdentifier<'a> for &'a str {
    type Reference = NodeOutputNameReference<'a>;

    fn combine(self, node_id: NodeId) -> Self::Reference {
        NodeOutputNameReference(node_id, self)
    }
}

pub trait InputPortReference: Copy {
    fn resolve<N: Node>(&self, graph: &Graph<N>) -> Option<InputPortId>;
}

impl InputPortReference for InputPortId {
    fn resolve<N: Node>(&self, _graph: &Graph<N>) -> Option<InputPortId> {
        Some(*self)
    }
}

pub trait OutputPortReference {
    fn resolve<N: Node>(&self, graph: &Graph<N>) -> Option<OutputPortId>;
}

impl OutputPortReference for OutputPortId {
    fn resolve<N: Node>(&self, _graph: &Graph<N>) -> Option<OutputPortId> {
        Some(*self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeInputIndexReference(NodeId, usize);

impl InputPortReference for NodeInputIndexReference {
    fn resolve<N: Node>(&self, graph: &Graph<N>) -> Option<InputPortId> {
        graph.get_input_port_at(self.0, self.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeOutputIndexReference(NodeId, usize);

impl OutputPortReference for NodeOutputIndexReference {
    fn resolve<N: Node>(&self, graph: &Graph<N>) -> Option<OutputPortId> {
        graph.get_output_port_at(self.0, self.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeInputNameReference<'a>(NodeId, &'a str);

impl<'a> InputPortReference for NodeInputNameReference<'a> {
    fn resolve<N: Node>(&self, graph: &Graph<N>) -> Option<InputPortId> {
        graph.get_input_port(self.0, self.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeOutputNameReference<'a>(NodeId, &'a str);

impl<'a> OutputPortReference for NodeOutputNameReference<'a> {
    fn resolve<N: Node>(&self, graph: &Graph<N>) -> Option<OutputPortId> {
        graph.get_output_port(self.0, self.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeInputDynamicReference<'a> {
    Index(NodeInputIndexReference),
    Name(NodeInputNameReference<'a>),
}

impl<'a> InputPortReference for NodeInputDynamicReference<'a> {
    fn resolve<N: Node>(&self, graph: &Graph<N>) -> Option<InputPortId> {
        match self {
            Self::Index(r) => r.resolve(graph),
            Self::Name(r) => r.resolve(graph),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeOutputDynamicReference<'a> {
    Index(NodeOutputIndexReference),
    Name(NodeOutputNameReference<'a>),
}

impl<'a> OutputPortReference for NodeOutputDynamicReference<'a> {
    fn resolve<N: Node>(&self, graph: &Graph<N>) -> Option<OutputPortId> {
        match self {
            Self::Index(r) => r.resolve(graph),
            Self::Name(r) => r.resolve(graph),
        }
    }
}
