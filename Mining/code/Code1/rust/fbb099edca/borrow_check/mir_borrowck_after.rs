fn mir_borrowck<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId) {
    let mir = tcx.mir_validated(def_id);
    let src = MirSource::from_local_def_id(tcx, def_id);
    debug!("run query mir_borrowck: {}", tcx.node_path_str(src.item_id()));

    let mir: &Mir<'tcx> = &mir.borrow();
    if !tcx.has_attr(def_id, "rustc_mir_borrowck") && !tcx.sess.opts.debugging_opts.borrowck_mir {
        return;
    }

    let id = src.item_id();
    let attributes = tcx.get_attrs(def_id);
    let param_env = tcx.param_env(def_id);
    tcx.infer_ctxt().enter(|_infcx| {

        let move_data = MoveData::gather_moves(mir, tcx, param_env);
        let mdpe = MoveDataParamEnv { move_data: move_data, param_env: param_env };
        let dead_unwinds = IdxSetBuf::new_empty(mir.basic_blocks().len());
        let flow_borrows = do_dataflow(tcx, mir, id, &attributes, &dead_unwinds,
                                       Borrows::new(tcx, mir),
                                       |bd, i| bd.location(i));
        let flow_inits = do_dataflow(tcx, mir, id, &attributes, &dead_unwinds,
                                     MaybeInitializedLvals::new(tcx, mir, &mdpe),
                                     |bd, i| &bd.move_data().move_paths[i]);
        let flow_uninits = do_dataflow(tcx, mir, id, &attributes, &dead_unwinds,
                                       MaybeUninitializedLvals::new(tcx, mir, &mdpe),
                                       |bd, i| &bd.move_data().move_paths[i]);

        let mut mbcx = MirBorrowckCtxt {
            tcx: tcx,
            mir: mir,
            node_id: id,
            move_data: &mdpe.move_data,
            param_env: param_env,
            fake_infer_ctxt: &_infcx,
        };

        let mut state = InProgress::new(flow_borrows,
                                        flow_inits,
                                        flow_uninits);

        mbcx.analyze_results(&mut state); // entry point for DataflowResultsConsumer
    });

    debug!("mir_borrowck done");
}

#[allow(dead_code)]
pub struct MirBorrowckCtxt<'c, 'b, 'a: 'b+'c, 'gcx: 'a+'tcx, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'gcx, 'gcx>,
    mir: &'b Mir<'gcx>,
    node_id: ast::NodeId,
    move_data: &'b MoveData<'gcx>,
    param_env: ParamEnv<'tcx>,
    fake_infer_ctxt: &'c InferCtxt<'c, 'gcx, 'tcx>,
}

// (forced to be `pub` due to its use as an associated type below.)
pub struct InProgress<'b, 'tcx: 'b> {
    borrows: FlowInProgress<Borrows<'b, 'tcx>>,
    inits: FlowInProgress<MaybeInitializedLvals<'b, 'tcx>>,
    uninits: FlowInProgress<MaybeUninitializedLvals<'b, 'tcx>>,
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
impl<'c, 'b, 'a: 'b+'c, 'gcx, 'tcx: 'a> DataflowResultsConsumer<'b, 'gcx>
    for MirBorrowckCtxt<'c, 'b, 'a, 'gcx, 'tcx>
{
    type FlowState = InProgress<'b, 'gcx>;

    fn mir(&self) -> &'b Mir<'gcx> { self.mir }

    fn reset_to_entry_of(&mut self, bb: BasicBlock, flow_state: &mut Self::FlowState) {
        flow_state.each_flow(|b| b.reset_to_entry_of(bb),
                             |i| i.reset_to_entry_of(bb),
                             |u| u.reset_to_entry_of(bb));
    }

    fn reconstruct_statement_effect(&mut self,
                                    location: Location,
                                    flow_state: &mut Self::FlowState) {
        flow_state.each_flow(|b| b.reconstruct_statement_effect(location),
                             |i| i.reconstruct_statement_effect(location),
                             |u| u.reconstruct_statement_effect(location));
    }

    fn apply_local_effect(&mut self,
                          _location: Location,
                          flow_state: &mut Self::FlowState) {
        flow_state.each_flow(|b| b.apply_local_effect(),
                             |i| i.apply_local_effect(),
                             |u| u.apply_local_effect());
    }

    fn reconstruct_terminator_effect(&mut self,
                                     location: Location,
                                     flow_state: &mut Self::FlowState) {
        flow_state.each_flow(|b| b.reconstruct_terminator_effect(location),
                             |i| i.reconstruct_terminator_effect(location),
                             |u| u.reconstruct_terminator_effect(location));
    }

    fn visit_block_entry(&mut self,
                         bb: BasicBlock,
                         flow_state: &Self::FlowState) {
        let summary = flow_state.summary();
        debug!("MirBorrowckCtxt::process_block({:?}): {}", bb, summary);
    }

    fn visit_statement_entry(&mut self,
                             location: Location,
                             stmt: &Statement<'gcx>,
                             flow_state: &Self::FlowState) {
        let summary = flow_state.summary();
        debug!("MirBorrowckCtxt::process_statement({:?}, {:?}): {}", location, stmt, summary);
        let span = stmt.source_info.span;
        match stmt.kind {
            StatementKind::Assign(ref lhs, ref rhs) => {
                self.mutate_lvalue(ContextKind::AssignLhs.new(location),
                                   (lhs, span), JustWrite, flow_state);
                self.consume_rvalue(ContextKind::AssignRhs.new(location),
                                    (rhs, span), location, flow_state);
            }
            StatementKind::SetDiscriminant { ref lvalue, variant_index: _ } => {
                self.mutate_lvalue(ContextKind::SetDiscrim.new(location),
                                   (lvalue, span), JustWrite, flow_state);
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
                // ignored by borrowck
            }

            StatementKind::StorageDead(ref lvalue) => {
                // causes non-drop values to be dropped.
                self.consume_lvalue(ContextKind::StorageDead.new(location),
                                    ConsumeKind::Consume,
                                    (lvalue, span),
                                    flow_state)
            }
        }
    }

    fn visit_terminator_entry(&mut self,
                              location: Location,
                              term: &Terminator<'gcx>,
                              flow_state: &Self::FlowState) {
        let loc = location;
        let summary = flow_state.summary();
        debug!("MirBorrowckCtxt::process_terminator({:?}, {:?}): {}", location, term, summary);
        let span = term.source_info.span;
        match term.kind {
            TerminatorKind::SwitchInt { ref discr, switch_ty: _, values: _, targets: _ } => {
                self.consume_operand(ContextKind::SwitchInt.new(loc),
                                     Consume,
                                     (discr, span), flow_state);
            }
            TerminatorKind::Drop { location: ref drop_lvalue, target: _, unwind: _ } => {
                self.consume_lvalue(ContextKind::Drop.new(loc),
                                    ConsumeKind::Drop,
                                    (drop_lvalue, span), flow_state);
            }
            TerminatorKind::DropAndReplace { location: ref drop_lvalue,
                                             value: ref new_value,
                                             target: _,
                                             unwind: _ } => {
                self.mutate_lvalue(ContextKind::DropAndReplace.new(loc),
                                   (drop_lvalue, span), JustWrite, flow_state);
                self.consume_operand(ContextKind::DropAndReplace.new(loc),
                                     ConsumeKind::Drop,
                                     (new_value, span), flow_state);
            }
            TerminatorKind::Call { ref func, ref args, ref destination, cleanup: _ } => {
                self.consume_operand(ContextKind::CallOperator.new(loc),
                                     Consume,
                                     (func, span), flow_state);
                for arg in args {
                    self.consume_operand(ContextKind::CallOperand.new(loc),
                                         Consume,
                                         (arg, span), flow_state);
                }
                if let Some((ref dest, _/*bb*/)) = *destination {
                    self.mutate_lvalue(ContextKind::CallDest.new(loc),
                                       (dest, span), JustWrite, flow_state);
                }
            }
            TerminatorKind::Assert { ref cond, expected: _, ref msg, target: _, cleanup: _ } => {
                self.consume_operand(ContextKind::Assert.new(loc),
                                     Consume,
                                     (cond, span), flow_state);
                match *msg {
                    AssertMessage::BoundsCheck { ref len, ref index } => {
                        self.consume_operand(ContextKind::Assert.new(loc),
                                             Consume,
                                             (len, span), flow_state);
                        self.consume_operand(ContextKind::Assert.new(loc),
                                             Consume,
                                             (index, span), flow_state);
                    }
                    AssertMessage::Math(_/*const_math_err*/) => {}
                }
            }

            TerminatorKind::Goto { target: _ } |
            TerminatorKind::Resume |
            TerminatorKind::Return |
            TerminatorKind::Unreachable => {
                // no data used, thus irrelevant to borrowck
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum MutateMode { JustWrite, WriteAndRead }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum ConsumeKind { Drop, Consume }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Control { Continue, Break }

impl<'c, 'b, 'a: 'b+'c, 'gcx, 'tcx: 'a> MirBorrowckCtxt<'c, 'b, 'a, 'gcx, 'tcx> {
    fn mutate_lvalue(&mut self,
                     context: Context,
                     lvalue_span: (&Lvalue<'gcx>, Span),
                     mode: MutateMode,
                     flow_state: &InProgress<'b, 'gcx>) {
        // Write of P[i] or *P, or WriteAndRead of any P, requires P init'd.
        match mode {
            MutateMode::WriteAndRead => {
                self.check_if_path_is_moved(context, lvalue_span, flow_state);
            }
            MutateMode::JustWrite => {
                self.check_if_assigned_path_is_moved(context, lvalue_span, flow_state);
            }
        }

        // check we don't invalidate any outstanding loans
        self.each_borrow_involving_path(context,
                                        lvalue_span.0, flow_state, |this, _index, _data| {
                                            this.report_illegal_mutation_of_borrowed(context,
                                                                                     lvalue_span);
                                            Control::Break
                                        });

        // check for reassignments to immutable local variables
        self.check_if_reassignment_to_immutable_state(context, lvalue_span, flow_state);
    }
