File_Code/databend/83da6a7841/plan_node/plan_node_after.rs --- Rust
188                 PlanNode::Empty(_) => {}                                                                                                                 188                 PlanNode::Fragment(_) => {
...                                                                                                                                                          189                     builder = builder.fragment()?;
...                                                                                                                                                          190                 }
...                                                                                                                                                          191                 // Non node in the list.
189                 PlanNode::Fragment(_) => {}                                                                                                              192                 PlanNode::Empty(_) => {}

