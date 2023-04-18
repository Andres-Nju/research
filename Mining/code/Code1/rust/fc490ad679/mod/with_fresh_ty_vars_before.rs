    pub fn with_fresh_ty_vars(selcx: &mut traits::SelectionContext<'a, 'gcx, 'tcx>,
                              impl_def_id: DefId)
                              -> ImplHeader<'tcx>
    {
        let tcx = selcx.tcx();
        let impl_substs = selcx.infcx().fresh_substs_for_item(DUMMY_SP, impl_def_id);

        let header = ImplHeader {
            impl_def_id: impl_def_id,
            self_ty: tcx.item_type(impl_def_id),
            trait_ref: tcx.impl_trait_ref(impl_def_id),
            predicates: tcx.item_predicates(impl_def_id).predicates
        }.subst(tcx, impl_substs);

        let traits::Normalized { value: mut header, obligations } =
            traits::normalize(selcx, traits::ObligationCause::dummy(), &header);

        header.predicates.extend(obligations.into_iter().map(|o| o.predicate));
        header
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AssociatedItem {
    pub def_id: DefId,
    pub name: Name,
    pub kind: AssociatedKind,
    pub vis: Visibility,
    pub defaultness: hir::Defaultness,
    pub container: AssociatedItemContainer,

    /// Whether this is a method with an explicit self
    /// as its first argument, allowing method calls.
    pub method_has_self_argument: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, RustcEncodable, RustcDecodable)]
pub enum AssociatedKind {
    Const,
    Method,
    Type
}

impl AssociatedItem {
    pub fn def(&self) -> Def {
        match self.kind {
            AssociatedKind::Const => Def::AssociatedConst(self.def_id),
            AssociatedKind::Method => Def::Method(self.def_id),
            AssociatedKind::Type => Def::AssociatedTy(self.def_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy, RustcEncodable, RustcDecodable)]
pub enum Visibility {
    /// Visible everywhere (including in other crates).
    Public,
    /// Visible only in the given crate-local module.
    Restricted(DefId),
    /// Not visible anywhere in the local crate. This is the visibility of private external items.
    Invisible,
}

pub trait DefIdTree: Copy {
    fn parent(self, id: DefId) -> Option<DefId>;

    fn is_descendant_of(self, mut descendant: DefId, ancestor: DefId) -> bool {
        if descendant.krate != ancestor.krate {
            return false;
        }

        while descendant != ancestor {
            match self.parent(descendant) {
                Some(parent) => descendant = parent,
                None => return false,
            }
        }
        true
    }
}

impl<'a, 'gcx, 'tcx> DefIdTree for TyCtxt<'a, 'gcx, 'tcx> {
    fn parent(self, id: DefId) -> Option<DefId> {
        self.def_key(id).parent.map(|index| DefId { index: index, ..id })
    }
}

impl Visibility {
    pub fn from_hir(visibility: &hir::Visibility, id: NodeId, tcx: TyCtxt) -> Self {
        match *visibility {
            hir::Public => Visibility::Public,
            hir::Visibility::Crate => Visibility::Restricted(DefId::local(CRATE_DEF_INDEX)),
            hir::Visibility::Restricted { ref path, .. } => match path.def {
                // If there is no resolution, `resolve` will have already reported an error, so
                // assume that the visibility is public to avoid reporting more privacy errors.
                Def::Err => Visibility::Public,
                def => Visibility::Restricted(def.def_id()),
            },
            hir::Inherited => {
                Visibility::Restricted(tcx.map.local_def_id(tcx.map.get_module_parent(id)))
            }
        }
    }

    /// Returns true if an item with this visibility is accessible from the given block.
    pub fn is_accessible_from<T: DefIdTree>(self, module: DefId, tree: T) -> bool {
        let restriction = match self {
            // Public items are visible everywhere.
            Visibility::Public => return true,
            // Private items from other crates are visible nowhere.
            Visibility::Invisible => return false,
            // Restricted items are visible in an arbitrary local module.
            Visibility::Restricted(other) if other.krate != module.krate => return false,
            Visibility::Restricted(module) => module,
        };

        tree.is_descendant_of(module, restriction)
    }

    /// Returns true if this visibility is at least as accessible as the given visibility
    pub fn is_at_least<T: DefIdTree>(self, vis: Visibility, tree: T) -> bool {
        let vis_restriction = match vis {
            Visibility::Public => return self == Visibility::Public,
            Visibility::Invisible => return true,
            Visibility::Restricted(module) => module,
        };

        self.is_accessible_from(vis_restriction, tree)
    }
}

#[derive(Clone, PartialEq, RustcDecodable, RustcEncodable, Copy)]
pub enum Variance {
    Covariant,      // T<A> <: T<B> iff A <: B -- e.g., function return type
    Invariant,      // T<A> <: T<B> iff B == A -- e.g., type of mutable cell
    Contravariant,  // T<A> <: T<B> iff B <: A -- e.g., function param type
    Bivariant,      // T<A> <: T<B>            -- e.g., unused type parameter
}

#[derive(Clone, Copy, Debug, RustcDecodable, RustcEncodable)]
pub struct MethodCallee<'tcx> {
    /// Impl method ID, for inherent methods, or trait method ID, otherwise.
    pub def_id: DefId,
    pub ty: Ty<'tcx>,
    pub substs: &'tcx Substs<'tcx>
}

/// With method calls, we store some extra information in
/// side tables (i.e method_map). We use
/// MethodCall as a key to index into these tables instead of
/// just directly using the expression's NodeId. The reason
/// for this being that we may apply adjustments (coercions)
/// with the resulting expression also needing to use the
/// side tables. The problem with this is that we don't
/// assign a separate NodeId to this new expression
/// and so it would clash with the base expression if both
/// needed to add to the side tables. Thus to disambiguate
/// we also keep track of whether there's an adjustment in
/// our key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct MethodCall {
    pub expr_id: NodeId,
    pub autoderef: u32
}

impl MethodCall {
    pub fn expr(id: NodeId) -> MethodCall {
        MethodCall {
            expr_id: id,
            autoderef: 0
        }
    }

    pub fn autoderef(expr_id: NodeId, autoderef: u32) -> MethodCall {
        MethodCall {
            expr_id: expr_id,
            autoderef: 1 + autoderef
        }
    }
}

// maps from an expression id that corresponds to a method call to the details
// of the method to be invoked
pub type MethodMap<'tcx> = FxHashMap<MethodCall, MethodCallee<'tcx>>;

// Contains information needed to resolve types and (in the future) look up
// the types of AST nodes.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct CReaderCacheKey {
    pub cnum: CrateNum,
    pub pos: usize,
}

/// Describes the fragment-state associated with a NodeId.
///
/// Currently only unfragmented paths have entries in the table,
/// but longer-term this enum is expected to expand to also
/// include data for fragmented paths.
#[derive(Copy, Clone, Debug)]
pub enum FragmentInfo {
    Moved { var: NodeId, move_expr: NodeId },
    Assigned { var: NodeId, assign_expr: NodeId, assignee_id: NodeId },
}

// Flags that we track on types. These flags are propagated upwards
// through the type during type construction, so that we can quickly
// check whether the type has various kinds of types in it without
// recursing over the type itself.
bitflags! {
    flags TypeFlags: u32 {
        const HAS_PARAMS         = 1 << 0,
        const HAS_SELF           = 1 << 1,
        const HAS_TY_INFER       = 1 << 2,
        const HAS_RE_INFER       = 1 << 3,
        const HAS_RE_SKOL        = 1 << 4,
        const HAS_RE_EARLY_BOUND = 1 << 5,
        const HAS_FREE_REGIONS   = 1 << 6,
        const HAS_TY_ERR         = 1 << 7,
        const HAS_PROJECTION     = 1 << 8,
        const HAS_TY_CLOSURE     = 1 << 9,

        // true if there are "names" of types and regions and so forth
        // that are local to a particular fn
        const HAS_LOCAL_NAMES    = 1 << 10,

        // Present if the type belongs in a local type context.
        // Only set for TyInfer other than Fresh.
        const KEEP_IN_LOCAL_TCX  = 1 << 11,

        // Is there a projection that does not involve a bound region?
        // Currently we can't normalize projections w/ bound regions.
        const HAS_NORMALIZABLE_PROJECTION = 1 << 12,

        const NEEDS_SUBST        = TypeFlags::HAS_PARAMS.bits |
                                   TypeFlags::HAS_SELF.bits |
                                   TypeFlags::HAS_RE_EARLY_BOUND.bits,

        // Flags representing the nominal content of a type,
        // computed by FlagsComputation. If you add a new nominal
        // flag, it should be added here too.
        const NOMINAL_FLAGS     = TypeFlags::HAS_PARAMS.bits |
                                  TypeFlags::HAS_SELF.bits |
                                  TypeFlags::HAS_TY_INFER.bits |
                                  TypeFlags::HAS_RE_INFER.bits |
                                  TypeFlags::HAS_RE_SKOL.bits |
                                  TypeFlags::HAS_RE_EARLY_BOUND.bits |
                                  TypeFlags::HAS_FREE_REGIONS.bits |
                                  TypeFlags::HAS_TY_ERR.bits |
                                  TypeFlags::HAS_PROJECTION.bits |
                                  TypeFlags::HAS_TY_CLOSURE.bits |
                                  TypeFlags::HAS_LOCAL_NAMES.bits |
                                  TypeFlags::KEEP_IN_LOCAL_TCX.bits,

        // Caches for type_is_sized, type_moves_by_default
        const SIZEDNESS_CACHED  = 1 << 16,
        const IS_SIZED          = 1 << 17,
        const MOVENESS_CACHED   = 1 << 18,
        const MOVES_BY_DEFAULT  = 1 << 19,
    }
}

pub struct TyS<'tcx> {
    pub sty: TypeVariants<'tcx>,
    pub flags: Cell<TypeFlags>,

    // the maximal depth of any bound regions appearing in this type.
    region_depth: u32,
}

impl<'tcx> PartialEq for TyS<'tcx> {
    #[inline]
    fn eq(&self, other: &TyS<'tcx>) -> bool {
        // (self as *const _) == (other as *const _)
        (self as *const TyS<'tcx>) == (other as *const TyS<'tcx>)
    }
}
impl<'tcx> Eq for TyS<'tcx> {}

impl<'tcx> Hash for TyS<'tcx> {
    fn hash<H: Hasher>(&self, s: &mut H) {
        (self as *const TyS).hash(s)
    }
}

pub type Ty<'tcx> = &'tcx TyS<'tcx>;

impl<'tcx> serialize::UseSpecializedEncodable for Ty<'tcx> {}
impl<'tcx> serialize::UseSpecializedDecodable for Ty<'tcx> {}

/// A wrapper for slices with the additional invariant
/// that the slice is interned and no other slice with
/// the same contents can exist in the same context.
/// This means we can use pointer + length for both
/// equality comparisons and hashing.
#[derive(Debug, RustcEncodable)]
pub struct Slice<T>([T]);

impl<T> PartialEq for Slice<T> {
    #[inline]
    fn eq(&self, other: &Slice<T>) -> bool {
        (&self.0 as *const [T]) == (&other.0 as *const [T])
    }
}
impl<T> Eq for Slice<T> {}

impl<T> Hash for Slice<T> {
    fn hash<H: Hasher>(&self, s: &mut H) {
        (self.as_ptr(), self.len()).hash(s)
    }
}

impl<T> Deref for Slice<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.0
    }
}

impl<'a, T> IntoIterator for &'a Slice<T> {
    type Item = &'a T;
    type IntoIter = <&'a [T] as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self[..].iter()
    }
}

impl<'tcx> serialize::UseSpecializedDecodable for &'tcx Slice<Ty<'tcx>> {}

impl<T> Slice<T> {
    pub fn empty<'a>() -> &'a Slice<T> {
        unsafe {
            mem::transmute(slice::from_raw_parts(0x1 as *const T, 0))
        }
    }
}

/// Upvars do not get their own node-id. Instead, we use the pair of
/// the original var id (that is, the root variable that is referenced
/// by the upvar) and the id of the closure expression.
#[derive(Clone, Copy, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable)]
pub struct UpvarId {
    pub var_id: NodeId,
    pub closure_expr_id: NodeId,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, RustcEncodable, RustcDecodable, Copy)]
pub enum BorrowKind {
    /// Data must be immutable and is aliasable.
    ImmBorrow,

    /// Data must be immutable but not aliasable.  This kind of borrow
    /// cannot currently be expressed by the user and is used only in
    /// implicit closure bindings. It is needed when you the closure
    /// is borrowing or mutating a mutable referent, e.g.:
    ///
    ///    let x: &mut isize = ...;
    ///    let y = || *x += 5;
    ///
    /// If we were to try to translate this closure into a more explicit
    /// form, we'd encounter an error with the code as written:
    ///
    ///    struct Env { x: & &mut isize }
    ///    let x: &mut isize = ...;
    ///    let y = (&mut Env { &x }, fn_ptr);  // Closure is pair of env and fn
    ///    fn fn_ptr(env: &mut Env) { **env.x += 5; }
    ///
    /// This is then illegal because you cannot mutate a `&mut` found
    /// in an aliasable location. To solve, you'd have to translate with
    /// an `&mut` borrow:
    ///
    ///    struct Env { x: & &mut isize }
    ///    let x: &mut isize = ...;
    ///    let y = (&mut Env { &mut x }, fn_ptr); // changed from &x to &mut x
    ///    fn fn_ptr(env: &mut Env) { **env.x += 5; }
    ///
    /// Now the assignment to `**env.x` is legal, but creating a
    /// mutable pointer to `x` is not because `x` is not mutable. We
    /// could fix this by declaring `x` as `let mut x`. This is ok in
    /// user code, if awkward, but extra weird for closures, since the
    /// borrow is hidden.
    ///
    /// So we introduce a "unique imm" borrow -- the referent is
    /// immutable, but not aliasable. This solves the problem. For
    /// simplicity, we don't give users the way to express this
    /// borrow, it's just used when translating closures.
    UniqueImmBorrow,

    /// Data is mutable and not aliasable.
    MutBorrow
}

/// Information describing the capture of an upvar. This is computed
/// during `typeck`, specifically by `regionck`.
#[derive(PartialEq, Clone, Debug, Copy, RustcEncodable, RustcDecodable)]
pub enum UpvarCapture<'tcx> {
    /// Upvar is captured by value. This is always true when the
    /// closure is labeled `move`, but can also be true in other cases
    /// depending on inference.
    ByValue,

    /// Upvar is captured by reference.
    ByRef(UpvarBorrow<'tcx>),
}

#[derive(PartialEq, Clone, Copy, RustcEncodable, RustcDecodable)]
pub struct UpvarBorrow<'tcx> {
    /// The kind of borrow: by-ref upvars have access to shared
    /// immutable borrows, which are not part of the normal language
    /// syntax.
    pub kind: BorrowKind,

    /// Region of the resulting reference.
    pub region: &'tcx ty::Region,
}

pub type UpvarCaptureMap<'tcx> = FxHashMap<UpvarId, UpvarCapture<'tcx>>;

#[derive(Copy, Clone)]
pub struct ClosureUpvar<'tcx> {
    pub def: Def,
    pub span: Span,
    pub ty: Ty<'tcx>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum IntVarValue {
    IntType(ast::IntTy),
    UintType(ast::UintTy),
}

/// Default region to use for the bound of objects that are
/// supplied as the value for this type parameter. This is derived
/// from `T:'a` annotations appearing in the type definition.  If
/// this is `None`, then the default is inherited from the
/// surrounding context. See RFC #599 for details.
#[derive(Copy, Clone, RustcEncodable, RustcDecodable)]
pub enum ObjectLifetimeDefault<'tcx> {
    /// Require an explicit annotation. Occurs when multiple
    /// `T:'a` constraints are found.
    Ambiguous,

    /// Use the base default, typically 'static, but in a fn body it is a fresh variable
    BaseDefault,

    /// Use the given region as the default.
    Specific(&'tcx Region),
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct TypeParameterDef<'tcx> {
    pub name: Name,
    pub def_id: DefId,
    pub index: u32,
    pub default_def_id: DefId, // for use in error reporing about defaults
    pub default: Option<Ty<'tcx>>,
    pub object_lifetime_default: ObjectLifetimeDefault<'tcx>,

    /// `pure_wrt_drop`, set by the (unsafe) `#[may_dangle]` attribute
    /// on generic parameter `T`, asserts data behind the parameter
    /// `T` won't be accessed during the parent type's `Drop` impl.
    pub pure_wrt_drop: bool,
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct RegionParameterDef<'tcx> {
    pub name: Name,
    pub def_id: DefId,
    pub index: u32,
    pub bounds: Vec<&'tcx ty::Region>,

    /// `pure_wrt_drop`, set by the (unsafe) `#[may_dangle]` attribute
    /// on generic parameter `'a`, asserts data of lifetime `'a`
    /// won't be accessed during the parent type's `Drop` impl.
    pub pure_wrt_drop: bool,
}

impl<'tcx> RegionParameterDef<'tcx> {
    pub fn to_early_bound_region_data(&self) -> ty::EarlyBoundRegion {
        ty::EarlyBoundRegion {
            index: self.index,
            name: self.name,
        }
    }

    pub fn to_bound_region(&self) -> ty::BoundRegion {
        // this is an early bound region, so unaffected by #32330
        ty::BoundRegion::BrNamed(self.def_id, self.name, Issue32330::WontChange)
    }
}

/// Information about the formal type/lifetime parameters associated
/// with an item or method. Analogous to hir::Generics.
#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct Generics<'tcx> {
    pub parent: Option<DefId>,
    pub parent_regions: u32,
    pub parent_types: u32,
    pub regions: Vec<RegionParameterDef<'tcx>>,
    pub types: Vec<TypeParameterDef<'tcx>>,
    pub has_self: bool,
}

impl<'tcx> Generics<'tcx> {
    pub fn parent_count(&self) -> usize {
        self.parent_regions as usize + self.parent_types as usize
    }

    pub fn own_count(&self) -> usize {
        self.regions.len() + self.types.len()
    }

    pub fn count(&self) -> usize {
        self.parent_count() + self.own_count()
    }

    pub fn region_param(&self, param: &EarlyBoundRegion) -> &RegionParameterDef<'tcx> {
        &self.regions[param.index as usize - self.has_self as usize]
    }

    pub fn type_param(&self, param: &ParamTy) -> &TypeParameterDef<'tcx> {
        &self.types[param.idx as usize - self.has_self as usize - self.regions.len()]
    }
}

/// Bounds on generics.
#[derive(Clone)]
pub struct GenericPredicates<'tcx> {
    pub parent: Option<DefId>,
    pub predicates: Vec<Predicate<'tcx>>,
}

impl<'tcx> serialize::UseSpecializedEncodable for GenericPredicates<'tcx> {}
impl<'tcx> serialize::UseSpecializedDecodable for GenericPredicates<'tcx> {}

impl<'a, 'gcx, 'tcx> GenericPredicates<'tcx> {
    pub fn instantiate(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>, substs: &Substs<'tcx>)
                       -> InstantiatedPredicates<'tcx> {
        let mut instantiated = InstantiatedPredicates::empty();
        self.instantiate_into(tcx, &mut instantiated, substs);
        instantiated
    }
    pub fn instantiate_own(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>, substs: &Substs<'tcx>)
                           -> InstantiatedPredicates<'tcx> {
        InstantiatedPredicates {
            predicates: self.predicates.subst(tcx, substs)
        }
    }

    fn instantiate_into(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                        instantiated: &mut InstantiatedPredicates<'tcx>,
                        substs: &Substs<'tcx>) {
        if let Some(def_id) = self.parent {
            tcx.item_predicates(def_id).instantiate_into(tcx, instantiated, substs);
        }
        instantiated.predicates.extend(self.predicates.iter().map(|p| p.subst(tcx, substs)))
    }

    pub fn instantiate_supertrait(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                                  poly_trait_ref: &ty::PolyTraitRef<'tcx>)
                                  -> InstantiatedPredicates<'tcx>
    {
        assert_eq!(self.parent, None);
        InstantiatedPredicates {
            predicates: self.predicates.iter().map(|pred| {
                pred.subst_supertrait(tcx, poly_trait_ref)
            }).collect()
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable)]
pub enum Predicate<'tcx> {
    /// Corresponds to `where Foo : Bar<A,B,C>`. `Foo` here would be
    /// the `Self` type of the trait reference and `A`, `B`, and `C`
    /// would be the type parameters.
    Trait(PolyTraitPredicate<'tcx>),

    /// where `T1 == T2`.
    Equate(PolyEquatePredicate<'tcx>),

    /// where 'a : 'b
    RegionOutlives(PolyRegionOutlivesPredicate<'tcx>),

    /// where T : 'a
    TypeOutlives(PolyTypeOutlivesPredicate<'tcx>),

    /// where <T as TraitRef>::Name == X, approximately.
    /// See `ProjectionPredicate` struct for details.
    Projection(PolyProjectionPredicate<'tcx>),

    /// no syntax: T WF
    WellFormed(Ty<'tcx>),

    /// trait must be object-safe
    ObjectSafe(DefId),

    /// No direct syntax. May be thought of as `where T : FnFoo<...>`
    /// for some substitutions `...` and T being a closure type.
    /// Satisfied (or refuted) once we know the closure's kind.
    ClosureKind(DefId, ClosureKind),
}

impl<'a, 'gcx, 'tcx> Predicate<'tcx> {
    /// Performs a substitution suitable for going from a
    /// poly-trait-ref to supertraits that must hold if that
    /// poly-trait-ref holds. This is slightly different from a normal
    /// substitution in terms of what happens with bound regions.  See
    /// lengthy comment below for details.
    pub fn subst_supertrait(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                            trait_ref: &ty::PolyTraitRef<'tcx>)
                            -> ty::Predicate<'tcx>
    {
        // The interaction between HRTB and supertraits is not entirely
        // obvious. Let me walk you (and myself) through an example.
        //
        // Let's start with an easy case. Consider two traits:
        //
        //     trait Foo<'a> : Bar<'a,'a> { }
        //     trait Bar<'b,'c> { }
        //
        // Now, if we have a trait reference `for<'x> T : Foo<'x>`, then
        // we can deduce that `for<'x> T : Bar<'x,'x>`. Basically, if we
        // knew that `Foo<'x>` (for any 'x) then we also know that
        // `Bar<'x,'x>` (for any 'x). This more-or-less falls out from
        // normal substitution.
        //
        // In terms of why this is sound, the idea is that whenever there
        // is an impl of `T:Foo<'a>`, it must show that `T:Bar<'a,'a>`
        // holds.  So if there is an impl of `T:Foo<'a>` that applies to
        // all `'a`, then we must know that `T:Bar<'a,'a>` holds for all
        // `'a`.
        //
        // Another example to be careful of is this:
        //
        //     trait Foo1<'a> : for<'b> Bar1<'a,'b> { }
        //     trait Bar1<'b,'c> { }
        //
        // Here, if we have `for<'x> T : Foo1<'x>`, then what do we know?
        // The answer is that we know `for<'x,'b> T : Bar1<'x,'b>`. The
        // reason is similar to the previous example: any impl of
        // `T:Foo1<'x>` must show that `for<'b> T : Bar1<'x, 'b>`.  So
        // basically we would want to collapse the bound lifetimes from
        // the input (`trait_ref`) and the supertraits.
        //
        // To achieve this in practice is fairly straightforward. Let's
        // consider the more complicated scenario:
        //
        // - We start out with `for<'x> T : Foo1<'x>`. In this case, `'x`
        //   has a De Bruijn index of 1. We want to produce `for<'x,'b> T : Bar1<'x,'b>`,
        //   where both `'x` and `'b` would have a DB index of 1.
        //   The substitution from the input trait-ref is therefore going to be
        //   `'a => 'x` (where `'x` has a DB index of 1).
        // - The super-trait-ref is `for<'b> Bar1<'a,'b>`, where `'a` is an
        //   early-bound parameter and `'b' is a late-bound parameter with a
        //   DB index of 1.
        // - If we replace `'a` with `'x` from the input, it too will have
        //   a DB index of 1, and thus we'll have `for<'x,'b> Bar1<'x,'b>`
        //   just as we wanted.
        //
        // There is only one catch. If we just apply the substitution `'a
        // => 'x` to `for<'b> Bar1<'a,'b>`, the substitution code will
        // adjust the DB index because we substituting into a binder (it
        // tries to be so smart...) resulting in `for<'x> for<'b>
        // Bar1<'x,'b>` (we have no syntax for this, so use your
        // imagination). Basically the 'x will have DB index of 2 and 'b
        // will have DB index of 1. Not quite what we want. So we apply
        // the substitution to the *contents* of the trait reference,
        // rather than the trait reference itself (put another way, the
        // substitution code expects equal binding levels in the values
        // from the substitution and the value being substituted into, and
        // this trick achieves that).

        let substs = &trait_ref.0.substs;
        match *self {
            Predicate::Trait(ty::Binder(ref data)) =>
                Predicate::Trait(ty::Binder(data.subst(tcx, substs))),
            Predicate::Equate(ty::Binder(ref data)) =>
                Predicate::Equate(ty::Binder(data.subst(tcx, substs))),
            Predicate::RegionOutlives(ty::Binder(ref data)) =>
                Predicate::RegionOutlives(ty::Binder(data.subst(tcx, substs))),
            Predicate::TypeOutlives(ty::Binder(ref data)) =>
                Predicate::TypeOutlives(ty::Binder(data.subst(tcx, substs))),
            Predicate::Projection(ty::Binder(ref data)) =>
                Predicate::Projection(ty::Binder(data.subst(tcx, substs))),
            Predicate::WellFormed(data) =>
                Predicate::WellFormed(data.subst(tcx, substs)),
            Predicate::ObjectSafe(trait_def_id) =>
                Predicate::ObjectSafe(trait_def_id),
            Predicate::ClosureKind(closure_def_id, kind) =>
                Predicate::ClosureKind(closure_def_id, kind),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable)]
pub struct TraitPredicate<'tcx> {
    pub trait_ref: TraitRef<'tcx>
}
pub type PolyTraitPredicate<'tcx> = ty::Binder<TraitPredicate<'tcx>>;

impl<'tcx> TraitPredicate<'tcx> {
    pub fn def_id(&self) -> DefId {
        self.trait_ref.def_id
    }

    /// Creates the dep-node for selecting/evaluating this trait reference.
    fn dep_node(&self) -> DepNode<DefId> {
        // Ideally, the dep-node would just have all the input types
        // in it.  But they are limited to including def-ids. So as an
        // approximation we include the def-ids for all nominal types
        // found somewhere. This means that we will e.g. conflate the
        // dep-nodes for `u32: SomeTrait` and `u64: SomeTrait`, but we
        // would have distinct dep-nodes for `Vec<u32>: SomeTrait`,
        // `Rc<u32>: SomeTrait`, and `(Vec<u32>, Rc<u32>): SomeTrait`.
        // Note that it's always sound to conflate dep-nodes, it just
        // leads to more recompilation.
        let def_ids: Vec<_> =
            self.input_types()
                .flat_map(|t| t.walk())
                .filter_map(|t| match t.sty {
                    ty::TyAdt(adt_def, _) =>
                        Some(adt_def.did),
                    _ =>
                        None
                })
                .chain(iter::once(self.def_id()))
                .collect();
        DepNode::TraitSelect(def_ids)
    }

    pub fn input_types<'a>(&'a self) -> impl DoubleEndedIterator<Item=Ty<'tcx>> + 'a {
        self.trait_ref.input_types()
    }

    pub fn self_ty(&self) -> Ty<'tcx> {
        self.trait_ref.self_ty()
    }
}

impl<'tcx> PolyTraitPredicate<'tcx> {
    pub fn def_id(&self) -> DefId {
        // ok to skip binder since trait def-id does not care about regions
        self.0.def_id()
    }

    pub fn dep_node(&self) -> DepNode<DefId> {
        // ok to skip binder since depnode does not care about regions
        self.0.dep_node()
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct EquatePredicate<'tcx>(pub Ty<'tcx>, pub Ty<'tcx>); // `0 == 1`
pub type PolyEquatePredicate<'tcx> = ty::Binder<EquatePredicate<'tcx>>;

#[derive(Clone, PartialEq, Eq, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct OutlivesPredicate<A,B>(pub A, pub B); // `A : B`
pub type PolyOutlivesPredicate<A,B> = ty::Binder<OutlivesPredicate<A,B>>;
pub type PolyRegionOutlivesPredicate<'tcx> = PolyOutlivesPredicate<&'tcx ty::Region,
                                                                   &'tcx ty::Region>;
pub type PolyTypeOutlivesPredicate<'tcx> = PolyOutlivesPredicate<Ty<'tcx>, &'tcx ty::Region>;

/// This kind of predicate has no *direct* correspondent in the
/// syntax, but it roughly corresponds to the syntactic forms:
///
/// 1. `T : TraitRef<..., Item=Type>`
/// 2. `<T as TraitRef<...>>::Item == Type` (NYI)
///
/// In particular, form #1 is "desugared" to the combination of a
/// normal trait predicate (`T : TraitRef<...>`) and one of these
/// predicates. Form #2 is a broader form in that it also permits
/// equality between arbitrary types. Processing an instance of Form
/// #2 eventually yields one of these `ProjectionPredicate`
/// instances to normalize the LHS.
#[derive(Copy, Clone, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable)]
pub struct ProjectionPredicate<'tcx> {
    pub projection_ty: ProjectionTy<'tcx>,
    pub ty: Ty<'tcx>,
}

pub type PolyProjectionPredicate<'tcx> = Binder<ProjectionPredicate<'tcx>>;

impl<'tcx> PolyProjectionPredicate<'tcx> {
    pub fn item_name(&self) -> Name {
        self.0.projection_ty.item_name // safe to skip the binder to access a name
    }
}

pub trait ToPolyTraitRef<'tcx> {
    fn to_poly_trait_ref(&self) -> PolyTraitRef<'tcx>;
}

impl<'tcx> ToPolyTraitRef<'tcx> for TraitRef<'tcx> {
    fn to_poly_trait_ref(&self) -> PolyTraitRef<'tcx> {
        assert!(!self.has_escaping_regions());
        ty::Binder(self.clone())
    }
}

impl<'tcx> ToPolyTraitRef<'tcx> for PolyTraitPredicate<'tcx> {
    fn to_poly_trait_ref(&self) -> PolyTraitRef<'tcx> {
        self.map_bound_ref(|trait_pred| trait_pred.trait_ref)
    }
}

impl<'tcx> ToPolyTraitRef<'tcx> for PolyProjectionPredicate<'tcx> {
    fn to_poly_trait_ref(&self) -> PolyTraitRef<'tcx> {
        // Note: unlike with TraitRef::to_poly_trait_ref(),
        // self.0.trait_ref is permitted to have escaping regions.
        // This is because here `self` has a `Binder` and so does our
        // return value, so we are preserving the number of binding
        // levels.
        ty::Binder(self.0.projection_ty.trait_ref)
    }
}

pub trait ToPredicate<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx>;
}

impl<'tcx> ToPredicate<'tcx> for TraitRef<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        // we're about to add a binder, so let's check that we don't
        // accidentally capture anything, or else that might be some
        // weird debruijn accounting.
        assert!(!self.has_escaping_regions());

        ty::Predicate::Trait(ty::Binder(ty::TraitPredicate {
            trait_ref: self.clone()
        }))
    }
}

impl<'tcx> ToPredicate<'tcx> for PolyTraitRef<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        ty::Predicate::Trait(self.to_poly_trait_predicate())
    }
}

impl<'tcx> ToPredicate<'tcx> for PolyEquatePredicate<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        Predicate::Equate(self.clone())
    }
}

impl<'tcx> ToPredicate<'tcx> for PolyRegionOutlivesPredicate<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        Predicate::RegionOutlives(self.clone())
    }
}

impl<'tcx> ToPredicate<'tcx> for PolyTypeOutlivesPredicate<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        Predicate::TypeOutlives(self.clone())
    }
}

impl<'tcx> ToPredicate<'tcx> for PolyProjectionPredicate<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        Predicate::Projection(self.clone())
    }
}

impl<'tcx> Predicate<'tcx> {
    /// Iterates over the types in this predicate. Note that in all
    /// cases this is skipping over a binder, so late-bound regions
    /// with depth 0 are bound by the predicate.
    pub fn walk_tys(&self) -> IntoIter<Ty<'tcx>> {
        let vec: Vec<_> = match *self {
            ty::Predicate::Trait(ref data) => {
                data.skip_binder().input_types().collect()
            }
            ty::Predicate::Equate(ty::Binder(ref data)) => {
                vec![data.0, data.1]
            }
            ty::Predicate::TypeOutlives(ty::Binder(ref data)) => {
                vec![data.0]
            }
            ty::Predicate::RegionOutlives(..) => {
                vec![]
            }
            ty::Predicate::Projection(ref data) => {
                let trait_inputs = data.0.projection_ty.trait_ref.input_types();
                trait_inputs.chain(Some(data.0.ty)).collect()
            }
            ty::Predicate::WellFormed(data) => {
                vec![data]
            }
            ty::Predicate::ObjectSafe(_trait_def_id) => {
                vec![]
            }
            ty::Predicate::ClosureKind(_closure_def_id, _kind) => {
                vec![]
            }
        };

        // The only reason to collect into a vector here is that I was
        // too lazy to make the full (somewhat complicated) iterator
        // type that would be needed here. But I wanted this fn to
        // return an iterator conceptually, rather than a `Vec`, so as
        // to be closer to `Ty::walk`.
        vec.into_iter()
    }

    pub fn to_opt_poly_trait_ref(&self) -> Option<PolyTraitRef<'tcx>> {
        match *self {
            Predicate::Trait(ref t) => {
                Some(t.to_poly_trait_ref())
            }
            Predicate::Projection(..) |
            Predicate::Equate(..) |
            Predicate::RegionOutlives(..) |
            Predicate::WellFormed(..) |
            Predicate::ObjectSafe(..) |
            Predicate::ClosureKind(..) |
            Predicate::TypeOutlives(..) => {
                None
            }
        }
    }
}

/// Represents the bounds declared on a particular set of type
/// parameters.  Should eventually be generalized into a flag list of
/// where clauses.  You can obtain a `InstantiatedPredicates` list from a
/// `GenericPredicates` by using the `instantiate` method. Note that this method
/// reflects an important semantic invariant of `InstantiatedPredicates`: while
/// the `GenericPredicates` are expressed in terms of the bound type
/// parameters of the impl/trait/whatever, an `InstantiatedPredicates` instance
/// represented a set of bounds for some particular instantiation,
/// meaning that the generic parameters have been substituted with
/// their values.
///
/// Example:
///
///     struct Foo<T,U:Bar<T>> { ... }
///
/// Here, the `GenericPredicates` for `Foo` would contain a list of bounds like
/// `[[], [U:Bar<T>]]`.  Now if there were some particular reference
/// like `Foo<isize,usize>`, then the `InstantiatedPredicates` would be `[[],
/// [usize:Bar<isize>]]`.
#[derive(Clone)]
pub struct InstantiatedPredicates<'tcx> {
    pub predicates: Vec<Predicate<'tcx>>,
}

impl<'tcx> InstantiatedPredicates<'tcx> {
    pub fn empty() -> InstantiatedPredicates<'tcx> {
        InstantiatedPredicates { predicates: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.predicates.is_empty()
    }
}

impl<'tcx> TraitRef<'tcx> {
    pub fn new(def_id: DefId, substs: &'tcx Substs<'tcx>) -> TraitRef<'tcx> {
        TraitRef { def_id: def_id, substs: substs }
    }

    pub fn self_ty(&self) -> Ty<'tcx> {
        self.substs.type_at(0)
    }

    pub fn input_types<'a>(&'a self) -> impl DoubleEndedIterator<Item=Ty<'tcx>> + 'a {
        // Select only the "input types" from a trait-reference. For
        // now this is all the types that appear in the
        // trait-reference, but it should eventually exclude
        // associated types.
        self.substs.types()
    }
}

/// When type checking, we use the `ParameterEnvironment` to track
/// details about the type/lifetime parameters that are in scope.
/// It primarily stores the bounds information.
///
/// Note: This information might seem to be redundant with the data in
/// `tcx.ty_param_defs`, but it is not. That table contains the
/// parameter definitions from an "outside" perspective, but this
/// struct will contain the bounds for a parameter as seen from inside
/// the function body. Currently the only real distinction is that
/// bound lifetime parameters are replaced with free ones, but in the
/// future I hope to refine the representation of types so as to make
/// more distinctions clearer.
#[derive(Clone)]
pub struct ParameterEnvironment<'tcx> {
    /// See `construct_free_substs` for details.
    pub free_substs: &'tcx Substs<'tcx>,

    /// Each type parameter has an implicit region bound that
    /// indicates it must outlive at least the function body (the user
    /// may specify stronger requirements). This field indicates the
    /// region of the callee.
    pub implicit_region_bound: &'tcx ty::Region,

    /// Obligations that the caller must satisfy. This is basically
    /// the set of bounds on the in-scope type parameters, translated
    /// into Obligations, and elaborated and normalized.
    pub caller_bounds: Vec<ty::Predicate<'tcx>>,

    /// Scope that is attached to free regions for this scope. This
    /// is usually the id of the fn body, but for more abstract scopes
    /// like structs we often use the node-id of the struct.
    ///
    /// FIXME(#3696). It would be nice to refactor so that free
    /// regions don't have this implicit scope and instead introduce
    /// relationships in the environment.
    pub free_id_outlive: CodeExtent,

    /// A cache for `moves_by_default`.
    pub is_copy_cache: RefCell<FxHashMap<Ty<'tcx>, bool>>,

    /// A cache for `type_is_sized`
    pub is_sized_cache: RefCell<FxHashMap<Ty<'tcx>, bool>>,
}

impl<'a, 'tcx> ParameterEnvironment<'tcx> {
    pub fn with_caller_bounds(&self,
                              caller_bounds: Vec<ty::Predicate<'tcx>>)
                              -> ParameterEnvironment<'tcx>
    {
        ParameterEnvironment {
            free_substs: self.free_substs,
            implicit_region_bound: self.implicit_region_bound,
            caller_bounds: caller_bounds,
            free_id_outlive: self.free_id_outlive,
            is_copy_cache: RefCell::new(FxHashMap()),
            is_sized_cache: RefCell::new(FxHashMap()),
        }
    }

    /// Construct a parameter environment given an item, impl item, or trait item
    pub fn for_item(tcx: TyCtxt<'a, 'tcx, 'tcx>, id: NodeId)
                    -> ParameterEnvironment<'tcx> {
        match tcx.map.find(id) {
            Some(ast_map::NodeImplItem(ref impl_item)) => {
                match impl_item.node {
                    hir::ImplItemKind::Type(_) | hir::ImplItemKind::Const(..) => {
                        // associated types don't have their own entry (for some reason),
                        // so for now just grab environment for the impl
                        let impl_id = tcx.map.get_parent(id);
                        let impl_def_id = tcx.map.local_def_id(impl_id);
                        tcx.construct_parameter_environment(impl_item.span,
                                                            impl_def_id,
                                                            tcx.region_maps.item_extent(id))
                    }
                    hir::ImplItemKind::Method(_, ref body) => {
                        tcx.construct_parameter_environment(
                            impl_item.span,
                            tcx.map.local_def_id(id),
                            tcx.region_maps.call_site_extent(id, body.node_id))
                    }
                }
            }
            Some(ast_map::NodeTraitItem(trait_item)) => {
                match trait_item.node {
                    hir::TraitItemKind::Type(..) | hir::TraitItemKind::Const(..) => {
                        // associated types don't have their own entry (for some reason),
                        // so for now just grab environment for the trait
                        let trait_id = tcx.map.get_parent(id);
                        let trait_def_id = tcx.map.local_def_id(trait_id);
                        tcx.construct_parameter_environment(trait_item.span,
                                                            trait_def_id,
                                                            tcx.region_maps.item_extent(id))
                    }
                    hir::TraitItemKind::Method(_, ref body) => {
                        // Use call-site for extent (unless this is a
                        // trait method with no default; then fallback
                        // to the method id).
                        let extent = if let hir::TraitMethod::Provided(body_id) = *body {
                            // default impl: use call_site extent as free_id_outlive bound.
                            tcx.region_maps.call_site_extent(id, body_id.node_id)
                        } else {
                            // no default impl: use item extent as free_id_outlive bound.
                            tcx.region_maps.item_extent(id)
                        };
                        tcx.construct_parameter_environment(
                            trait_item.span,
                            tcx.map.local_def_id(id),
                            extent)
                    }
                }
            }
            Some(ast_map::NodeItem(item)) => {
                match item.node {
                    hir::ItemFn(.., body_id) => {
                        // We assume this is a function.
                        let fn_def_id = tcx.map.local_def_id(id);

                        tcx.construct_parameter_environment(
                            item.span,
                            fn_def_id,
                            tcx.region_maps.call_site_extent(id, body_id.node_id))
                    }
                    hir::ItemEnum(..) |
                    hir::ItemStruct(..) |
                    hir::ItemUnion(..) |
                    hir::ItemTy(..) |
                    hir::ItemImpl(..) |
                    hir::ItemConst(..) |
                    hir::ItemStatic(..) => {
                        let def_id = tcx.map.local_def_id(id);
                        tcx.construct_parameter_environment(item.span,
                                                            def_id,
                                                            tcx.region_maps.item_extent(id))
                    }
                    hir::ItemTrait(..) => {
                        let def_id = tcx.map.local_def_id(id);
                        tcx.construct_parameter_environment(item.span,
                                                            def_id,
                                                            tcx.region_maps.item_extent(id))
                    }
                    _ => {
                        span_bug!(item.span,
                                  "ParameterEnvironment::for_item():
                                   can't create a parameter \
                                   environment for this kind of item")
                    }
                }
            }
            Some(ast_map::NodeExpr(expr)) => {
                // This is a convenience to allow closures to work.
                if let hir::ExprClosure(.., body, _) = expr.node {
                    let def_id = tcx.map.local_def_id(id);
                    let base_def_id = tcx.closure_base_def_id(def_id);
                    tcx.construct_parameter_environment(
                        expr.span,
                        base_def_id,
                        tcx.region_maps.call_site_extent(id, body.node_id))
                } else {
                    tcx.empty_parameter_environment()
                }
            }
            Some(ast_map::NodeForeignItem(item)) => {
                let def_id = tcx.map.local_def_id(id);
                tcx.construct_parameter_environment(item.span,
                                                    def_id,
                                                    ROOT_CODE_EXTENT)
            }
            _ => {
                bug!("ParameterEnvironment::from_item(): \
                      `{}` is not an item",
                     tcx.map.node_to_string(id))
            }
        }
    }
}

bitflags! {
    flags AdtFlags: u32 {
        const NO_ADT_FLAGS        = 0,
        const IS_ENUM             = 1 << 0,
        const IS_DTORCK           = 1 << 1, // is this a dtorck type?
        const IS_DTORCK_VALID     = 1 << 2,
        const IS_PHANTOM_DATA     = 1 << 3,
        const IS_SIMD             = 1 << 4,
        const IS_FUNDAMENTAL      = 1 << 5,
        const IS_UNION            = 1 << 6,
    }
}

pub struct VariantDef {
    /// The variant's DefId. If this is a tuple-like struct,
    /// this is the DefId of the struct's ctor.
    pub did: DefId,
    pub name: Name, // struct's name if this is a struct
    pub disr_val: Disr,
    pub fields: Vec<FieldDef>,
    pub ctor_kind: CtorKind,
}

pub struct FieldDef {
    pub did: DefId,
    pub name: Name,
    pub vis: Visibility,
}

/// The definition of an abstract data type - a struct or enum.
///
/// These are all interned (by intern_adt_def) into the adt_defs
/// table.
pub struct AdtDef {
    pub did: DefId,
    pub variants: Vec<VariantDef>,
    destructor: Cell<Option<DefId>>,
    flags: Cell<AdtFlags>
}

impl PartialEq for AdtDef {
    // AdtDef are always interned and this is part of TyS equality
    #[inline]
    fn eq(&self, other: &Self) -> bool { self as *const _ == other as *const _ }
}

impl Eq for AdtDef {}

impl Hash for AdtDef {
    #[inline]
    fn hash<H: Hasher>(&self, s: &mut H) {
        (self as *const AdtDef).hash(s)
    }
}

impl<'tcx> serialize::UseSpecializedEncodable for &'tcx AdtDef {
    fn default_encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        self.did.encode(s)
    }
}

impl<'tcx> serialize::UseSpecializedDecodable for &'tcx AdtDef {}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AdtKind { Struct, Union, Enum }

impl<'a, 'gcx, 'tcx> AdtDef {
    fn new(tcx: TyCtxt<'a, 'gcx, 'tcx>,
           did: DefId,
           kind: AdtKind,
           variants: Vec<VariantDef>) -> Self {
        let mut flags = AdtFlags::NO_ADT_FLAGS;
        let attrs = tcx.get_attrs(did);
        if attr::contains_name(&attrs, "fundamental") {
            flags = flags | AdtFlags::IS_FUNDAMENTAL;
        }
        if tcx.lookup_simd(did) {
            flags = flags | AdtFlags::IS_SIMD;
        }
        if Some(did) == tcx.lang_items.phantom_data() {
            flags = flags | AdtFlags::IS_PHANTOM_DATA;
        }
        match kind {
            AdtKind::Enum => flags = flags | AdtFlags::IS_ENUM,
            AdtKind::Union => flags = flags | AdtFlags::IS_UNION,
            AdtKind::Struct => {}
        }
        AdtDef {
            did: did,
            variants: variants,
            flags: Cell::new(flags),
            destructor: Cell::new(None),
        }
    }
