# `node_graph`

Simple framework for applications that use node graphs. Example use cases:

- Visual Shaders
- Visual Programming
- Modular Synthesis
- Circuit Simulation

This crate provides simple data structures that allow for general usage, it is
not a complete visual programming or circuit simulation crate, however making an
application using `node_graph` that can do those things is significantly easier.
The crate also provides a rudimentary analyzer and graph walker to aid with the
use cases. The graph walker will go over each node in the graph, guaranteeing
that previous/dependency nodes are executed first.

Check the examples and cargo docs for information on how to use the crate.

PRs are welcome but keep your AI garbage to yourself.
