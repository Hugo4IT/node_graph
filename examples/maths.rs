use node_graph::{
    Graph, InitialPorts, Node,
    walker::{GraphWalkContext, GraphWalker},
};

#[derive(Debug, Clone, Copy)]
enum MyNode {
    Constant(f32),
    Multiply,
    Print,
}

impl MyNode {
    fn evaluate(&mut self, context: &mut GraphWalkContext<Self>) {
        match self {
            Self::Constant(value) => context.set(0, *value),
            Self::Multiply => context.set(0, context.get(0) * context.get(1)),
            Self::Print => println!("{}", context.get(0)),
        }
    }
}

impl Node for MyNode {
    type DataType = ();
    type DataValue = f32;

    fn initial_ports(&self) -> InitialPorts<Self> {
        match self {
            Self::Constant(_) => InitialPorts {
                outputs: vec![("value", ())],
                ..Default::default()
            },
            Self::Multiply => InitialPorts {
                inputs: vec![("a", (), 0.0), ("b", (), 0.0)],
                outputs: vec![("result", ())],
            },
            Self::Print => InitialPorts {
                inputs: vec![("value", (), 0.0)],
                ..Default::default()
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
