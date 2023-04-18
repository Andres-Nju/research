fn place_element_conflict<'a, 'gcx: 'tcx, 'tcx>(
    tcx: TyCtxt<'a, 'gcx, 'tcx>,
    mir: &Mir<'tcx>,
    elem1: &Place<'tcx>,
    elem2: &Place<'tcx>,
) -> Overlap {
    match (elem1, elem2) {
        (Place::Local(l1), Place::Local(l2)) => {
            if l1 == l2 {
                // the same local - base case, equal
                debug!("place_element_conflict: DISJOINT-OR-EQ-LOCAL");
                Overlap::EqualOrDisjoint
            } else {
                // different locals - base case, disjoint
                debug!("place_element_conflict: DISJOINT-LOCAL");
                Overlap::Disjoint
            }
        }
        (Place::Static(static1), Place::Static(static2)) => {
            if static1.def_id != static2.def_id {
                debug!("place_element_conflict: DISJOINT-STATIC");
                Overlap::Disjoint
            } else if tcx.is_static(static1.def_id) == Some(hir::Mutability::MutMutable) {
                // We ignore mutable statics - they can only be unsafe code.
                debug!("place_element_conflict: IGNORE-STATIC-MUT");
                Overlap::Disjoint
            } else {
                debug!("place_element_conflict: DISJOINT-OR-EQ-STATIC");
                Overlap::EqualOrDisjoint
            }
        }
        (Place::Local(_), Place::Static(_)) | (Place::Static(_), Place::Local(_)) => {
            debug!("place_element_conflict: DISJOINT-STATIC-LOCAL");
            Overlap::Disjoint
        }
        (Place::Projection(pi1), Place::Projection(pi2)) => {
            match (&pi1.elem, &pi2.elem) {
                (ProjectionElem::Deref, ProjectionElem::Deref) => {
                    // derefs (e.g. `*x` vs. `*x`) - recur.
                    debug!("place_element_conflict: DISJOINT-OR-EQ-DEREF");
                    Overlap::EqualOrDisjoint
                }
                (ProjectionElem::Field(f1, _), ProjectionElem::Field(f2, _)) => {
                    if f1 == f2 {
                        // same field (e.g. `a.y` vs. `a.y`) - recur.
                        debug!("place_element_conflict: DISJOINT-OR-EQ-FIELD");
                        Overlap::EqualOrDisjoint
                    } else {
                        let ty = pi1.base.ty(mir, tcx).to_ty(tcx);
                        match ty.sty {
                            ty::TyAdt(def, _) if def.is_union() => {
                                // Different fields of a union, we are basically stuck.
                                debug!("place_element_conflict: STUCK-UNION");
                                Overlap::Arbitrary
                            }
                            _ => {
                                // Different fields of a struct (`a.x` vs. `a.y`). Disjoint!
                                debug!("place_element_conflict: DISJOINT-FIELD");
                                Overlap::Disjoint
                            }
                        }
                    }
                }
                (ProjectionElem::Downcast(_, v1), ProjectionElem::Downcast(_, v2)) => {
                    // different variants are treated as having disjoint fields,
                    // even if they occupy the same "space", because it's
                    // impossible for 2 variants of the same enum to exist
                    // (and therefore, to be borrowed) at the same time.
                    //
                    // Note that this is different from unions - we *do* allow
                    // this code to compile:
                    //
                    // ```
                    // fn foo(x: &mut Result<i32, i32>) {
                    //     let mut v = None;
                    //     if let Ok(ref mut a) = *x {
                    //         v = Some(a);
                    //     }
                    //     // here, you would *think* that the
                    //     // *entirety* of `x` would be borrowed,
                    //     // but in fact only the `Ok` variant is,
                    //     // so the `Err` variant is *entirely free*:
                    //     if let Err(ref mut a) = *x {
                    //         v = Some(a);
                    //     }
                    //     drop(v);
                    // }
                    // ```
                    if v1 == v2 {
                        debug!("place_element_conflict: DISJOINT-OR-EQ-FIELD");
                        Overlap::EqualOrDisjoint
                    } else {
                        debug!("place_element_conflict: DISJOINT-FIELD");
                        Overlap::Disjoint
                    }
                }
                (ProjectionElem::Index(..), ProjectionElem::Index(..))
                | (ProjectionElem::Index(..), ProjectionElem::ConstantIndex { .. })
                | (ProjectionElem::Index(..), ProjectionElem::Subslice { .. })
                | (ProjectionElem::ConstantIndex { .. }, ProjectionElem::Index(..))
                | (ProjectionElem::Subslice { .. }, ProjectionElem::Index(..)) => {
                    // Array indexes (`a[0]` vs. `a[i]`). These can either be disjoint
                    // (if the indexes differ) or equal (if they are the same), so this
                    // is the recursive case that gives "equal *or* disjoint" its meaning.
                    debug!("place_element_conflict: DISJOINT-OR-EQ-ARRAY-INDEX");
                    Overlap::EqualOrDisjoint
                }
                (ProjectionElem::ConstantIndex { offset: o1, min_length: _, from_end: false },
                    ProjectionElem::ConstantIndex { offset: o2, min_length: _, from_end: false })
                | (ProjectionElem::ConstantIndex { offset: o1, min_length: _, from_end: true },
                    ProjectionElem::ConstantIndex {
                        offset: o2, min_length: _, from_end: true }) => {
                    if o1 == o2 {
                        debug!("place_element_conflict: DISJOINT-OR-EQ-ARRAY-CONSTANT-INDEX");
                        Overlap::EqualOrDisjoint
                    } else {
                        debug!("place_element_conflict: DISJOINT-ARRAY-CONSTANT-INDEX");
                        Overlap::Disjoint
                    }
                }
                (ProjectionElem::ConstantIndex {
                    offset: offset_from_begin, min_length: min_length1, from_end: false },
                    ProjectionElem::ConstantIndex {
                        offset: offset_from_end, min_length: min_length2, from_end: true })
                | (ProjectionElem::ConstantIndex {
                    offset: offset_from_end, min_length: min_length1, from_end: true },
                   ProjectionElem::ConstantIndex {
                       offset: offset_from_begin, min_length: min_length2, from_end: false }) => {
                    // both patterns matched so it must be at least the greater of the two
                    let min_length = max(min_length1, min_length2);
                    // offset_from_end can be in range [1..min_length], -1 for last and min_length
                    // for first, min_length - offset_from_end gives minimal possible offset from
                    // the beginning
                    if *offset_from_begin >= min_length - offset_from_end {
                        debug!("place_element_conflict: DISJOINT-OR-EQ-ARRAY-CONSTANT-INDEX-FE");
                        Overlap::EqualOrDisjoint
                    } else {
                        debug!("place_element_conflict: DISJOINT-ARRAY-CONSTANT-INDEX-FE");
                        Overlap::Disjoint
                    }
                }
                (ProjectionElem::ConstantIndex { offset, min_length: _, from_end: false },
                 ProjectionElem::Subslice {from, .. })
                | (ProjectionElem::Subslice {from, .. },
                    ProjectionElem::ConstantIndex { offset, min_length: _, from_end: false }) => {
                    if offset >= from {
                        debug!(
                            "place_element_conflict: DISJOINT-OR-EQ-ARRAY-CONSTANT-INDEX-SUBSLICE");
                        Overlap::EqualOrDisjoint
                    } else {
                        debug!("place_element_conflict: DISJOINT-ARRAY-CONSTANT-INDEX-SUBSLICE");
                        Overlap::Disjoint
                    }
                }
                (ProjectionElem::ConstantIndex { offset, min_length: _, from_end: true },
                 ProjectionElem::Subslice {from: _, to })
                | (ProjectionElem::Subslice {from: _, to },
                    ProjectionElem::ConstantIndex { offset, min_length: _, from_end: true }) => {
                    if offset > to {
                        debug!("place_element_conflict: \
                               DISJOINT-OR-EQ-ARRAY-CONSTANT-INDEX-SUBSLICE-FE");
                        Overlap::EqualOrDisjoint
                    } else {
                        debug!("place_element_conflict: DISJOINT-ARRAY-CONSTANT-INDEX-SUBSLICE-FE");
                        Overlap::Disjoint
                    }
                }
                (ProjectionElem::Subslice { .. }, ProjectionElem::Subslice { .. }) => {
                    debug!("place_element_conflict: DISJOINT-OR-EQ-ARRAY-SUBSLICES");
                     Overlap::EqualOrDisjoint
                }
                (ProjectionElem::Deref, _)
                | (ProjectionElem::Field(..), _)
                | (ProjectionElem::Index(..), _)
                | (ProjectionElem::ConstantIndex { .. }, _)
                | (ProjectionElem::Subslice { .. }, _)
                | (ProjectionElem::Downcast(..), _) => bug!(
                    "mismatched projections in place_element_conflict: {:?} and {:?}",
                    elem1,
                    elem2
                ),
            }
        }
        (Place::Projection(_), _) | (_, Place::Projection(_)) => bug!(
            "unexpected elements in place_element_conflict: {:?} and {:?}",
            elem1,
            elem2
        ),
    }
}
