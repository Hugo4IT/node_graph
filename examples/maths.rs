use node_graph::{
    DataType, DataValue, Graph, InitialPorts, Node,
    walker::{GraphWalkContext, GraphWalker},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    // We only use floats in this example
    Float,
}

impl DataType for Type {}

// Simple wrapper around f32 as that is the only type needed, but this can be
// replaced with an enum of multiple types of values
#[derive(Debug, Clone, Copy, Default)]
#[repr(transparent)]
pub struct Value(pub f32);

impl DataValue<Type> for Value {
    // Not used currently
    fn ty(&self) -> Type {
        Type::Float
    }

    // Not used currently
    fn default_for(_ty: Type) -> Self {
        Value(0.0)
    }

    // When multiple output ports are connected to a single input, this function
    // will combine them into a single value for the node to use
    fn flatten<'a>(values: impl Iterator<Item = &'a Self>) -> Self {
        Self(values.map(|v| v.0).sum())
    }
}

#[derive(Debug, Clone, Copy)]
enum MyNode {
    Constant(f32),
    Multiply,
    Print,
}

impl MyNode {
    fn evaluate(&mut self, context: &mut GraphWalkContext<Self>) {
        match self {
            Self::Constant(value) => context.set(0, Value(*value)),
            Self::Multiply => context.set(0, Value(context.get(0).0 * context.get(1).0)),
            Self::Print => println!("{}", context.get(0).0),
        }
    }
}

impl Node for MyNode {
    type DataType = Type;
    type DataValue = Value;

    fn initial_ports(&self) -> InitialPorts<Self> {
        match self {
            Self::Constant(_) => InitialPorts {
                inputs: &[],
                outputs: &[("value", Type::Float)],
            },
            Self::Multiply => InitialPorts {
                inputs: &[("a", Type::Float), ("b", Type::Float)],
                outputs: &[("result", Type::Float)],
            },
            Self::Print => InitialPorts {
                inputs: &[("value", Type::Float)],
                outputs: &[],
            },
        }
    }
}

fn main() {
    let mut graph = Graph::new();

    let constant_1 = graph.create_node(MyNode::Constant(5.0));
    let constant_2 = graph.create_node(MyNode::Constant(7.0));
    let multiply = graph.create_node(MyNode::Multiply);
    let print = graph.create_node(MyNode::Print);

    graph.connect(constant_1.output(0), multiply.input("a"));
    graph.connect(constant_2.output(0), multiply.input("b"));
    graph.connect(multiply.output(0), print.input(0));

    GraphWalker::new(&graph, None).walk(MyNode::evaluate);
}
