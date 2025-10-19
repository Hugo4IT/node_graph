pub mod analyzer;
pub mod macros;
pub mod reference;
pub mod walker;

use std::fmt::Debug;

use itertools::Itertools;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use slotmap::{Key, SecondaryMap, SlotMap, new_key_type};

use crate::reference::{
    InputPortReference, NodeInputIdentifier, NodeOutputIdentifier, OutputPortReference,
};

pub(crate) const INVALID_STATE: &str = "Graph is in invalid state, this is a bug";

new_key_type! { pub struct NodeId; }
new_key_type! { pub struct ConnectionId; }
new_key_type! { pub struct InputPortId; }
new_key_type! { pub struct OutputPortId; }

impl NodeId {
    pub fn input<'a, I: NodeInputIdentifier<'a>>(&self, identifier: I) -> I::Reference {
        identifier.combine(*self)
    }

    pub fn output<'a, I: NodeOutputIdentifier<'a>>(&self, identifier: I) -> I::Reference {
        identifier.combine(*self)
    }
}

#[derive(Debug)]
pub struct Graph<N: Node> {
    node_data: SlotMap<NodeId, NodeData>,
    nodes: SecondaryMap<NodeId, RwLock<N>>,
    connections: SlotMap<ConnectionId, Connection>,
    input_ports: SlotMap<InputPortId, Port<N>>,
    output_ports: SlotMap<OutputPortId, Port<N>>,
}

impl<N: Node> Graph<N> {
    pub fn new() -> Self {
        Self {
            node_data: SlotMap::with_key(),
            nodes: SecondaryMap::new(),
            connections: SlotMap::with_key(),
            input_ports: SlotMap::with_key(),
            output_ports: SlotMap::with_key(),
        }
    }

    pub fn get_node(&self, node: NodeId) -> Option<RwLockReadGuard<'_, N>> {
        Some(self.nodes.get(node)?.read())
    }

    pub fn get_node_mut(&self, node: NodeId) -> Option<RwLockWriteGuard<'_, N>> {
        Some(self.nodes.get(node)?.write())
    }

    pub fn get_input_port_info(&self, port: impl InputPortReference) -> Option<&Port<N>> {
        self.input_ports.get(port.resolve(&self)?)
    }

    pub fn get_output_port_info(&self, port: impl OutputPortReference) -> Option<&Port<N>> {
        self.output_ports.get(port.resolve(&self)?)
    }

    pub fn create_node<T: NodeTemplate<N>>(&mut self, node: T) -> NodeId {
        let (node, callback) = node.split();
        let initial_ports = node.initial_ports();

        let id = self.node_data.insert_with_key(|node_id| {
            let mut node_data = NodeData::default();

            // Add node initial ports

            for &(name, ty, ref default) in initial_ports.inputs.iter() {
                let id = self.input_ports.insert(Port::new(
                    node_id,
                    name.to_string(),
                    ty,
                    Some(default.clone()),
                ));

                node_data.inputs.push((name.to_string(), id));
            }

            for &(name, ty) in initial_ports.outputs.iter() {
                let id = self
                    .output_ports
                    .insert(Port::new(node_id, name.to_string(), ty, None));

                node_data.outputs.push((name.to_string(), id));
            }

            node_data
        });

        self.nodes.insert(id, RwLock::new(node));

        callback.post_create(self, id);

        id
    }

    pub fn create_node_with<T: NodeTemplate<N>, const INPUTS: usize, const OUTPUTS: usize>(
        &mut self,
        node: T,
        inputs: [(&str, N::DataType, N::DataValue); INPUTS],
        outputs: [(&str, N::DataType); OUTPUTS],
    ) -> (NodeId, [InputPortId; INPUTS], [OutputPortId; OUTPUTS]) {
        let (node, callback) = node.split();
        let initial_ports = node.initial_ports();

        let mut input_ports = [InputPortId::null(); INPUTS];
        let mut output_ports = [OutputPortId::null(); OUTPUTS];

        let id = self.node_data.insert_with_key(|node_id| {
            let mut node_data = NodeData::default();

            // First add initial ports

            for &(name, ty, ref default) in initial_ports.inputs.iter() {
                let id = self.input_ports.insert(Port::new(
                    node_id,
                    name.to_string(),
                    ty,
                    Some(default.clone()),
                ));

                node_data.inputs.push((name.to_string(), id));
            }

            for &(name, ty) in initial_ports.outputs.iter() {
                let id = self
                    .output_ports
                    .insert(Port::new(node_id, name.to_string(), ty, None));

                node_data.outputs.push((name.to_string(), id));
            }

            // Then add user ports

            for (i, (name, ty, default)) in inputs.into_iter().enumerate() {
                let id = self.input_ports.insert(Port::new(
                    node_id,
                    name.to_string(),
                    ty,
                    Some(default),
                ));

                node_data.inputs.push((name.to_string(), id));
                input_ports[i] = id;
            }

            for (i, (name, ty)) in outputs.into_iter().enumerate() {
                let id = self
                    .output_ports
                    .insert(Port::new(node_id, name.to_string(), ty, None));

                node_data.outputs.push((name.to_string(), id));
                output_ports[i] = id;
            }

            node_data
        });

        self.nodes.insert(id, RwLock::new(node));

        callback.post_create(self, id);

        (id, input_ports, output_ports)
    }

    pub fn create_input_port(
        &mut self,
        node: NodeId,
        name: &str,
        ty: N::DataType,
        default: N::DataValue,
    ) -> InputPortId {
        let data = self.node_data.get_mut(node).expect("Node does not exist");

        if data.inputs.iter().any(|(port_name, _)| port_name == name) {
            panic!("An input port with this name already exists");
        }

        let id = self
            .input_ports
            .insert(Port::new(node, name.to_string(), ty, Some(default)));

        data.inputs.push((name.to_string(), id));

        let node = self.nodes.get(node).expect("Node does not exist");
        node.write().input_port_created(name, ty, id);

        id
    }

    pub fn create_output_port(
        &mut self,
        node: NodeId,
        name: &str,
        ty: N::DataType,
    ) -> OutputPortId {
        let data = self.node_data.get_mut(node).expect("Node does not exist");

        if data.outputs.iter().any(|(port_name, _)| port_name == name) {
            panic!("An output port with this name already exists");
        }

        let id = self
            .output_ports
            .insert(Port::new(node, name.to_string(), ty, None));

        data.outputs.push((name.to_string(), id));

        let node = self.nodes.get(node).expect("Node does not exist");
        node.write().output_port_created(name, ty, id);

        id
    }

    #[must_use]
    pub fn delete_input_port(&mut self, port: impl InputPortReference) -> Option<()> {
        let port = port.resolve(&self)?;

        let mut port = self.input_ports.remove(port)?;

        // Disconnect everything from port

        for connection_id in port.incoming_connections.drain(..) {
            let Some(connection) = self.connections.remove(connection_id) else {
                continue;
            };

            let start_port = self
                .output_ports
                .get_mut(connection.start_port)
                .expect(INVALID_STATE);

            start_port.outgoing_connections.remove(
                start_port
                    .outgoing_connections
                    .iter()
                    .position(|&id| id == connection_id)
                    .expect(INVALID_STATE),
            );

            let start_node_id = start_port.node;

            let start_node = self.nodes.get(start_node_id).expect(INVALID_STATE);

            start_node
                .write()
                .output_connection_removed(connection.start_port, connection_id);
        }

        Some(())
    }

    #[must_use]
    pub fn delete_output_port(&mut self, port: impl OutputPortReference) -> Option<()> {
        let port = port.resolve(&self)?;

        let mut port = self.output_ports.remove(port)?;

        // Disconnect everything from port

        for connection_id in port.outgoing_connections.drain(..) {
            let Some(connection) = self.connections.remove(connection_id) else {
                continue;
            };

            let end_port = self
                .input_ports
                .get_mut(connection.end_port)
                .expect(INVALID_STATE);

            end_port.incoming_connections.remove(
                end_port
                    .incoming_connections
                    .iter()
                    .position(|&id| id == connection_id)
                    .expect(INVALID_STATE),
            );

            let end_node_id = end_port.node;

            let end_node = self.nodes.get(end_node_id).expect(INVALID_STATE);

            end_node
                .write()
                .input_connection_removed(connection.end_port, connection_id);
        }

        Some(())
    }

    pub fn get_input_port(&self, node: NodeId, name: &str) -> Option<InputPortId> {
        let node = self.node_data.get(node)?;

        node.inputs
            .iter()
            .find_map(|(port_name, port_id)| (port_name == name).then_some(*port_id))
    }

    pub fn get_output_port(&self, node: NodeId, name: &str) -> Option<OutputPortId> {
        let node = self.node_data.get(node)?;

        node.outputs
            .iter()
            .find_map(|(port_name, port_id)| (port_name == name).then_some(*port_id))
    }

    pub fn get_input_port_at(&self, node: NodeId, index: usize) -> Option<InputPortId> {
        let node = self.node_data.get(node)?;

        Some(node.inputs.get(index)?.1)
    }

    pub fn get_output_port_at(&self, node: NodeId, index: usize) -> Option<OutputPortId> {
        let node = self.node_data.get(node)?;

        Some(node.outputs.get(index)?.1)
    }

    pub fn get_input_ports(&self, node: NodeId) -> Option<&Vec<(String, InputPortId)>> {
        let node = self.node_data.get(node)?;

        Some(&node.inputs)
    }

    pub fn set_default_value(
        &mut self,
        port: impl InputPortReference,
        value: impl Into<N::DataValue>,
    ) {
        let value: N::DataValue = value.into();

        let port = self
            .input_ports
            .get_mut(port.resolve(&self).expect("Port does not exist"))
            .expect("Input port does not exist");

        port.default = Some(value);
    }

    pub fn get_output_ports(&self, node: NodeId) -> Option<&Vec<(String, OutputPortId)>> {
        let node = self.node_data.get(node)?;

        Some(&node.outputs)
    }

    pub fn get_incoming_connections(
        &self,
        port: impl InputPortReference,
    ) -> impl Iterator<Item = OutputPortId> + '_ {
        let port = port.resolve(&self).expect("Port does not exist");
        let port = self
            .input_ports
            .get(port)
            .expect("Input port does not exist");

        port.incoming_connections.iter().map(|&conn_id| {
            self.connections
                .get(conn_id)
                .expect(INVALID_STATE)
                .start_port
        })
    }

    pub fn get_outgoing_connections(
        &self,
        port: impl OutputPortReference,
    ) -> impl Iterator<Item = InputPortId> + '_ {
        let port = port.resolve(&self).expect("Port does not exist");
        let port = self
            .output_ports
            .get(port)
            .expect("Output port does not exist");

        port.outgoing_connections
            .iter()
            .map(|&conn_id| self.connections.get(conn_id).expect(INVALID_STATE).end_port)
    }

    pub fn get_direct_dependencies(&self, node: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let node = self.node_data.get(node).expect("Node does not exist");

        node.inputs
            .iter()
            .flat_map(|(_, id)| {
                self.input_ports
                    .get(*id)
                    .expect(INVALID_STATE)
                    .incoming_connections
                    .iter()
                    .map(|&conn_id| {
                        self.output_ports
                            .get(
                                self.connections
                                    .get(conn_id)
                                    .expect(INVALID_STATE)
                                    .start_port,
                            )
                            .expect(INVALID_STATE)
                            .node
                    })
            })
            .unique()
    }

    pub fn can_connect(
        &self,
        start_port: impl OutputPortReference,
        end_port: impl InputPortReference,
    ) -> bool {
        let start_port = start_port
            .resolve(&self)
            .expect("Start port does not exist");

        let end_port = end_port.resolve(&self).expect("End port does not exist");

        let start = self
            .output_ports
            .get(start_port)
            .expect("Start port of connection does not exist");

        let end = self
            .input_ports
            .get(end_port)
            .expect("End port of connection does not exist");

        start.node != end.node && start.ty.can_convert_to(end.ty)
    }

    pub fn connect(
        &mut self,
        start_port: impl OutputPortReference,
        end_port: impl InputPortReference,
    ) -> ConnectionId {
        let start_port = start_port
            .resolve(&self)
            .expect("Start port does not exist");

        let end_port = end_port.resolve(&self).expect("End port does not exist`");

        let connection = Connection {
            start_port,
            end_port,
        };

        let id = self.connections.insert(connection);

        let start = self
            .output_ports
            .get_mut(start_port)
            .expect("Start port of connection does not exist");

        start.outgoing_connections.push(id);

        let start_ty = start.ty;
        let start_node_id = start.node;

        let start_node = self.nodes.get(start.node).expect(INVALID_STATE);

        start_node.write().output_connection_added(start_port, id);

        let end = self
            .input_ports
            .get_mut(end_port)
            .expect("End port of connection does not exist");

        if start_node_id == end.node {
            panic!("Attempted to create a connection to the same node");
        }

        if !start_ty.can_convert_to(end.ty) {
            panic!("Attempted to create a connection between two ports of non-convertable types");
        }

        end.incoming_connections.push(id);

        let end_node = self.nodes.get(end.node).expect(INVALID_STATE);

        end_node.write().input_connection_added(end_port, id);

        id
    }
}

impl<N: Node> Default for Graph<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Node + PartialEq> Graph<N> {
    pub fn find<'a>(&'a self, node: &'a N) -> impl Iterator<Item = NodeId> + 'a {
        self.nodes
            .iter()
            .filter_map(|(id, other)| other.read().eq(node).then_some(id))
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeData {
    inputs: Vec<(String, InputPortId)>,
    outputs: Vec<(String, OutputPortId)>,
}

#[derive(Debug, Clone)]
pub struct InitialPorts<N: Node> {
    pub inputs: Vec<(&'static str, N::DataType, N::DataValue)>,
    pub outputs: Vec<(&'static str, N::DataType)>,
}

impl<N: Node> Default for InitialPorts<N> {
    fn default() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
}

pub trait Node: Sized + 'static {
    type DataType: DataType;
    type DataValue: Debug + Clone + 'static;

    fn initial_ports(&self) -> InitialPorts<Self> {
        Default::default()
    }

    fn input_port_created(&mut self, name: &str, ty: Self::DataType, id: InputPortId) {
        let _ = (name, ty, id);
    }

    fn input_connection_added(&mut self, port: InputPortId, connection: ConnectionId) {
        let _ = (port, connection);
    }

    fn input_connection_removed(&mut self, port: InputPortId, connection: ConnectionId) {
        let _ = (port, connection);
    }

    fn output_port_created(&mut self, name: &str, ty: Self::DataType, id: OutputPortId) {
        let _ = (name, ty, id);
    }

    fn output_connection_added(&mut self, port: OutputPortId, connection: ConnectionId) {
        let _ = (port, connection);
    }

    fn output_connection_removed(&mut self, port: OutputPortId, connection: ConnectionId) {
        let _ = (port, connection);
    }
}

pub trait DataType: Debug + Clone + Copy + Eq {
    fn can_convert_to(&self, rhs: Self) -> bool {
        *self == rhs
    }
}

impl DataType for () {}

#[derive(Debug, Clone, Copy)]
pub struct Connection {
    start_port: OutputPortId,
    end_port: InputPortId,
}

#[derive(Debug, Clone, Default)]
pub struct Port<N: Node> {
    pub node: NodeId,
    pub name: String,
    pub ty: N::DataType,
    pub default: Option<N::DataValue>,
    pub incoming_connections: Vec<ConnectionId>,
    pub outgoing_connections: Vec<ConnectionId>,
}

impl<N: Node> Port<N> {
    pub fn new(node: NodeId, name: String, ty: N::DataType, default: Option<N::DataValue>) -> Self {
        Self {
            node,
            name,
            ty,
            default,
            incoming_connections: Vec::new(),
            outgoing_connections: Vec::new(),
        }
    }
}

pub trait NodeTemplate<N: Node> {
    type Callback: NodeTemplateCallback<N>;

    fn split(self) -> (N, Self::Callback);
}

pub trait NodeTemplateCallback<N: Node> {
    fn post_create(self, graph: &mut Graph<N>, node_id: NodeId);
}

#[derive(Debug, Clone, Copy)]
pub struct EmptyNodeCallback;

impl<N: Node> NodeTemplateCallback<N> for EmptyNodeCallback {
    fn post_create(self, graph: &mut Graph<N>, node_id: NodeId) {
        let _ = (graph, node_id);
    }
}

impl<T: Into<N>, N: Node> NodeTemplate<N> for T {
    type Callback = EmptyNodeCallback;

    fn split(self) -> (N, EmptyNodeCallback) {
        (self.into(), EmptyNodeCallback)
    }
}
