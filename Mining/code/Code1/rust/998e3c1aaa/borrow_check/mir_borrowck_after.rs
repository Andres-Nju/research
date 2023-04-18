fn mir_borrowck<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId) {
    let input_mir = tcx.mir_validated(def_id);
    debug!("run query mir_borrowck: {}", tcx.item_path_str(def_id));

    if {
        !tcx.has_attr(def_id, "rustc_mir_borrowck") &&
            !tcx.sess.opts.debugging_opts.borrowck_mir &&
            !tcx.sess.opts.debugging_opts.nll
    } {
        return;
    }

    tcx.infer_ctxt().enter(|infcx| {
        let input_mir: &Mir = &input_mir.borrow();
        do_mir_borrowck(&infcx, input_mir, def_id);
    });
    debug!("mir_borrowck done");
}

fn do_mir_borrowck<'a, 'gcx, 'tcx>(infcx: &InferCtxt<'a, 'gcx, 'tcx>,
                                   input_mir: &Mir<'gcx>,
                                   def_id: DefId)
{
    let tcx = infcx.tcx;
    let attributes = tcx.get_attrs(def_id);
    let param_env = tcx.param_env(def_id);
    let id = tcx.hir.as_local_node_id(def_id)
        .expect("do_mir_borrowck: non-local DefId");

    let move_data: MoveData<'tcx> = match MoveData::gather_moves(input_mir, tcx, param_env) {
        Ok(move_data) => move_data,
        Err((move_data, move_errors)) => {
            for move_error in move_errors {
                let (span, kind): (Span, IllegalMoveOriginKind) = match move_error {
                    MoveError::UnionMove { .. } =>
                        unimplemented!("dont know how to report union move errors yet."),
                    MoveError::IllegalMove { cannot_move_out_of: o } => (o.span, o.kind),
                };
                let origin = Origin::Mir;
                let mut err = match kind {
                    IllegalMoveOriginKind::Static =>
                        tcx.cannot_move_out_of(span, "static item", origin),
                    IllegalMoveOriginKind::BorrowedContent =>
                        tcx.cannot_move_out_of(span, "borrowed content", origin),
                    IllegalMoveOriginKind::InteriorOfTypeWithDestructor { container_ty: ty } =>
                        tcx.cannot_move_out_of_interior_of_drop(span, ty, origin),
                    IllegalMoveOriginKind::InteriorOfSliceOrArray { ty, is_index } =>
                        tcx.cannot_move_out_of_interior_noncopy(span, ty, is_index, origin),
                };
                err.emit();
            }
            move_data
        }
    };

    // Make our own copy of the MIR. This copy will be modified (in place) to
    // contain non-lexical lifetimes. It will have a lifetime tied
    // to the inference context.
    let mut mir: Mir<'tcx> = input_mir.clone();
    let mir = &mut mir;

    // If we are in non-lexical mode, compute the non-lexical lifetimes.
    let opt_regioncx = if !tcx.sess.opts.debugging_opts.nll {
        None
    } else {
        Some(nll::compute_regions(infcx, def_id, mir))
    };

    let mdpe = MoveDataParamEnv { move_data: move_data, param_env: param_env };
    let dead_unwinds = IdxSetBuf::new_empty(mir.basic_blocks().len());
    let flow_borrows = do_dataflow(tcx, mir, id, &attributes, &dead_unwinds,
                                   Borrows::new(tcx, mir, opt_regioncx.as_ref()),
                                   |bd, i| bd.location(i));
    let flow_inits = do_dataflow(tcx, mir, id, &attributes, &dead_unwinds,
                                 MaybeInitializedLvals::new(tcx, mir, &mdpe),
                                 |bd, i| &bd.move_data().move_paths[i]);
    let flow_uninits = do_dataflow(tcx, mir, id, &attributes, &dead_unwinds,
                                   MaybeUninitializedLvals::new(tcx, mir, &mdpe),
                                   |bd, i| &bd.move_data().move_paths[i]);
    let flow_move_outs = do_dataflow(tcx, mir, id, &attributes, &dead_unwinds,
                                     MovingOutStatements::new(tcx, mir, &mdpe),
                                     |bd, i| &bd.move_data().moves[i]);

    let mut mbcx = MirBorrowckCtxt {
        tcx: tcx,
        mir: mir,
        node_id: id,
        move_data: &mdpe.move_data,
        param_env: param_env,
        fake_infer_ctxt: &infcx,
    };

    let mut state = InProgress::new(flow_borrows,
                                    flow_inits,
                                    flow_uninits,
                                    flow_move_outs);

    mbcx.analyze_results(&mut state); // entry point for DataflowResultsConsumer
}

#[allow(dead_code)]
pub struct MirBorrowckCtxt<'c, 'b, 'a: 'b+'c, 'gcx: 'a+'tcx, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'gcx, 'tcx>,
    mir: &'b Mir<'tcx>,
    node_id: ast::NodeId,
    move_data: &'b MoveData<'tcx>,
    param_env: ParamEnv<'tcx>,
    fake_infer_ctxt: &'c InferCtxt<'c, 'gcx, 'tcx>,
}

// (forced to be `pub` due to its use as an associated type below.)
pub struct InProgress<'b, 'gcx: 'tcx, 'tcx: 'b> {
    borrows: FlowInProgress<Borrows<'b, 'gcx, 'tcx>>,
    inits: FlowInProgress<MaybeInitializedLvals<'b, 'gcx, 'tcx>>,
    uninits: FlowInProgress<MaybeUninitializedLvals<'b, 'gcx, 'tcx>>,
    move_outs: FlowInProgress<MovingOutStatements<'b, 'gcx, 'tcx>>,
}

struct FlowInProgress<BD> where BD: BitDenotation {
    base_results: DataflowResults<BD>,
    curr_state: IdxSetBuf<BD::Idx>,
    stmt_gen: IdxSetBuf<BD::Idx>,
    stmt_kill: IdxSetBuf<BD::Idx>,
}

// Check that:
// 1. assignments are always made to mutable locations (FIXME: does that still really go here?)
// 2. loans made in overlapping scopes do not conflict
// 3. assignments do not affect things loaned out as immutable
// 4. moves do not affect things loaned out in any way
impl<'c, 'b, 'a: 'b+'c, 'gcx, 'tcx: 'a> DataflowResultsConsumer<'b, 'tcx>
    for MirBorrowckCtxt<'c, 'b, 'a, 'gcx, 'tcx>
{
    type FlowState = InProgress<'b, 'gcx, 'tcx>;

    fn mir(&self) -> &'b Mir<'tcx> { self.mir }

    fn reset_to_entry_of(&mut self, bb: BasicBlock, flow_state: &mut Self::FlowState) {
        flow_state.each_flow(|b| b.reset_to_entry_of(bb),
                             |i| i.reset_to_entry_of(bb),
                             |u| u.reset_to_entry_of(bb),
                             |m| m.reset_to_entry_of(bb));
    }

    fn reconstruct_statement_effect(&mut self,
                                    location: Location,
                                    flow_state: &mut Self::FlowState) {
        flow_state.each_flow(|b| b.reconstruct_statement_effect(location),
                             |i| i.reconstruct_statement_effect(location),
                             |u| u.reconstruct_statement_effect(location),
                             |m| m.reconstruct_statement_effect(location));
    }

    fn apply_local_effect(&mut self,
                          _location: Location,
                          flow_state: &mut Self::FlowState) {
        flow_state.each_flow(|b| b.apply_local_effect(),
                             |i| i.apply_local_effect(),
                             |u| u.apply_local_effect(),
                             |m| m.apply_local_effect());
    }

    fn reconstruct_terminator_effect(&mut self,
                                     location: Location,
                                     flow_state: &mut Self::FlowState) {
        flow_state.each_flow(|b| b.reconstruct_terminator_effect(location),
                             |i| i.reconstruct_terminator_effect(location),
                             |u| u.reconstruct_terminator_effect(location),
                             |m| m.reconstruct_terminator_effect(location));
    }

    fn visit_block_entry(&mut self,
                         bb: BasicBlock,
                         flow_state: &Self::FlowState) {
        let summary = flow_state.summary();
        debug!("MirBorrowckCtxt::process_block({:?}): {}", bb, summary);
    }

    fn visit_statement_entry(&mut self,
                             location: Location,
                             stmt: &Statement<'tcx>,
                             flow_state: &Self::FlowState) {
        let summary = flow_state.summary();
        debug!("MirBorrowckCtxt::process_statement({:?}, {:?}): {}", location, stmt, summary);
        let span = stmt.source_info.span;
        match stmt.kind {
            StatementKind::Assign(ref lhs, ref rhs) => {
                // NOTE: NLL RFC calls for *shallow* write; using Deep
                // for short-term compat w/ AST-borrowck. Also, switch
                // to shallow requires to dataflow: "if this is an
                // assignment `lv = <rvalue>`, then any loan for some
                // path P of which `lv` is a prefix is killed."
                self.mutate_lvalue(ContextKind::AssignLhs.new(location),
                                   (lhs, span), Deep, JustWrite, flow_state);

                self.consume_rvalue(ContextKind::AssignRhs.new(location),
                                    (rhs, span), location, flow_state);
            }
            StatementKind::SetDiscriminant { ref lvalue, variant_index: _ } => {
                self.mutate_lvalue(ContextKind::SetDiscrim.new(location),
                                   (lvalue, span),
                                   Shallow(Some(ArtificialField::Discriminant)),
                                   JustWrite,
                                   flow_state);
            }
            StatementKind::InlineAsm { ref asm, ref outputs, ref inputs } => {
                for (o, output) in asm.outputs.iter().zip(outputs) {
                    if o.is_indirect {
                        self.consume_lvalue(ContextKind::InlineAsm.new(location),
                                            Consume,
                                            (output, span),
                                            flow_state);
                    } else {
                        self.mutate_lvalue(ContextKind::InlineAsm.new(location),
                                           (output, span),
                                           Deep,
                                           if o.is_rw { WriteAndRead } else { JustWrite },
                                           flow_state);
                    }
                }
                for input in inputs {
                    self.consume_operand(ContextKind::InlineAsm.new(location),
                                         Consume,
                                         (input, span), flow_state);
                }
            }
            StatementKind::EndRegion(ref _rgn) => {
                // ignored when consuming results (update to
                // flow_state already handled).
            }
            StatementKind::Nop |
            StatementKind::Validate(..) |
            StatementKind::StorageLive(..) => {
                // `Nop`, `Validate`, and `StorageLive` are irrelevant
                // to borrow check.
            }

            StatementKind::StorageDead(local) => {
                self.access_lvalue(ContextKind::StorageDead.new(location),
                                   (&Lvalue::Local(local), span),
                                   (Shallow(None), Write(WriteKind::StorageDead)),
                                   flow_state);
            }
        }
    }
