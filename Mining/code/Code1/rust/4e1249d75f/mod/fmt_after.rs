    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        use self::Rvalue::*;

        match *self {
            Use(ref lvalue) => write!(fmt, "{:?}", lvalue),
            Repeat(ref a, ref b) => write!(fmt, "[{:?}; {:?}]", a, b),
            Len(ref a) => write!(fmt, "Len({:?})", a),
            Cast(ref kind, ref lv, ref ty) => write!(fmt, "{:?} as {:?} ({:?})", lv, ty, kind),
            BinaryOp(ref op, ref a, ref b) => write!(fmt, "{:?}({:?}, {:?})", op, a, b),
            CheckedBinaryOp(ref op, ref a, ref b) => {
                write!(fmt, "Checked{:?}({:?}, {:?})", op, a, b)
            }
            UnaryOp(ref op, ref a) => write!(fmt, "{:?}({:?})", op, a),
            Discriminant(ref lval) => write!(fmt, "discriminant({:?})", lval),
            NullaryOp(ref op, ref t) => write!(fmt, "{:?}({:?})", op, t),
            Ref(region, borrow_kind, ref lv) => {
                let kind_str = match borrow_kind {
                    BorrowKind::Shared => "",
                    BorrowKind::Mut | BorrowKind::Unique => "mut ",
                };

                // When printing regions, add trailing space if necessary.
                let region = if ppaux::verbose() || ppaux::identify_regions() {
                    let mut region = format!("{}", region);
                    if region.len() > 0 { region.push(' '); }
                    region
                } else {
                    // Do not even print 'static
                    "".to_owned()
                };
                write!(fmt, "&{}{}{:?}", region, kind_str, lv)
            }

            Aggregate(ref kind, ref lvs) => {
                fn fmt_tuple(fmt: &mut Formatter, lvs: &[Operand]) -> fmt::Result {
                    let mut tuple_fmt = fmt.debug_tuple("");
                    for lv in lvs {
                        tuple_fmt.field(lv);
                    }
                    tuple_fmt.finish()
                }

                match **kind {
                    AggregateKind::Array(_) => write!(fmt, "{:?}", lvs),

                    AggregateKind::Tuple => {
                        match lvs.len() {
                            0 => write!(fmt, "()"),
                            1 => write!(fmt, "({:?},)", lvs[0]),
                            _ => fmt_tuple(fmt, lvs),
                        }
                    }

                    AggregateKind::Adt(adt_def, variant, substs, _) => {
                        let variant_def = &adt_def.variants[variant];

                        ppaux::parameterized(fmt, substs, variant_def.did, &[])?;

                        match variant_def.ctor_kind {
                            CtorKind::Const => Ok(()),
                            CtorKind::Fn => fmt_tuple(fmt, lvs),
                            CtorKind::Fictive => {
                                let mut struct_fmt = fmt.debug_struct("");
                                for (field, lv) in variant_def.fields.iter().zip(lvs) {
                                    struct_fmt.field(&field.name.as_str(), lv);
                                }
                                struct_fmt.finish()
                            }
                        }
                    }

                    AggregateKind::Closure(def_id, _) => ty::tls::with(|tcx| {
                        if let Some(node_id) = tcx.hir.as_local_node_id(def_id) {
                            let name = if tcx.sess.opts.debugging_opts.span_free_formats {
                                format!("[closure@{:?}]", node_id)
                            } else {
                                format!("[closure@{:?}]", tcx.hir.span(node_id))
                            };
                            let mut struct_fmt = fmt.debug_struct(&name);

                            tcx.with_freevars(node_id, |freevars| {
                                for (freevar, lv) in freevars.iter().zip(lvs) {
                                    let def_id = freevar.def.def_id();
                                    let var_id = tcx.hir.as_local_node_id(def_id).unwrap();
                                    let var_name = tcx.local_var_name_str(var_id);
                                    struct_fmt.field(&var_name, lv);
                                }
                            });

                            struct_fmt.finish()
                        } else {
                            write!(fmt, "[closure]")
                        }
                    }),
                }
            }
        }
    }
