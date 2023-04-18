    pub fn plan_list_to_node(
        ctx: FuseQueryContextRef,
        list: &[PlanNode],
    ) -> FuseQueryResult<PlanNode> {
        let mut builder = PlanBuilder::empty(ctx.clone());
        for plan in list {
            match plan {
                PlanNode::Projection(v) => {
                    builder = builder.project(v.expr.clone())?;
                }
                PlanNode::AggregatorPartial(v) => {
                    builder =
                        builder.aggregate_partial(v.aggr_expr.clone(), v.group_expr.clone())?;
                }
                PlanNode::AggregatorFinal(v) => {
                    builder = builder.aggregate_final(v.aggr_expr.clone(), v.group_expr.clone())?;
                }
                PlanNode::Filter(v) => {
                    builder = builder.filter(v.predicate.clone())?;
                }
                PlanNode::Limit(v) => {
                    builder = builder.limit(v.n)?;
                }
                PlanNode::ReadSource(v) => {
                    builder = PlanBuilder::from(ctx.clone(), &PlanNode::ReadSource(v.clone()))
                }
                PlanNode::Explain(_v) => {
                    builder = builder.explain()?;
                }
                PlanNode::Select(_v) => {
                    builder = builder.select()?;
                }
                PlanNode::Empty(_) => {}
                PlanNode::Fragment(_) => {}
                PlanNode::Scan(_) => {}
                PlanNode::SetVariable(_) => {}
            }
        }
        builder.build()
    }
