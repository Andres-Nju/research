    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// hack to ensure that we don't try to access the private parts of `ItemLocalId` in this module
mod item_local_id_inner {
    use rustc_data_structures::indexed_vec::Idx;
    use rustc_macros::HashStable;
    newtype_index! {
        /// An `ItemLocalId` uniquely identifies something within a given "item-like",
        /// that is, within a hir::Item, hir::TraitItem, or hir::ImplItem. There is no
        /// guarantee that the numerical value of a given `ItemLocalId` corresponds to
        /// the node's position within the owning item in any way, but there is a
        /// guarantee that the `LocalItemId`s within an owner occupy a dense range of
        /// integers starting at zero, so a mapping that maps all or most nodes within
        /// an "item-like" to something else can be implemented by a `Vec` instead of a
        /// tree or hash map.
        pub struct ItemLocalId {
            derive [HashStable]
        }
    }
}

pub use self::item_local_id_inner::ItemLocalId;

/// The `HirId` corresponding to `CRATE_NODE_ID` and `CRATE_DEF_INDEX`.
pub const CRATE_HIR_ID: HirId = HirId {
    owner: CRATE_DEF_INDEX,
    local_id: ItemLocalId::from_u32_const(0)
};

pub const DUMMY_HIR_ID: HirId = HirId {
    owner: CRATE_DEF_INDEX,
    local_id: DUMMY_ITEM_LOCAL_ID,
};

pub const DUMMY_ITEM_LOCAL_ID: ItemLocalId = ItemLocalId::MAX;

#[derive(Clone, RustcEncodable, RustcDecodable, Copy, HashStable)]
pub struct Lifetime {
    pub hir_id: HirId,
    pub span: Span,

    /// Either "`'a`", referring to a named lifetime definition,
    /// or "``" (i.e., `keywords::Invalid`), for elision placeholders.
    ///
    /// HIR lowering inserts these placeholders in type paths that
    /// refer to type definitions needing lifetime parameters,
    /// `&T` and `&mut T`, and trait objects without `... + 'a`.
    pub name: LifetimeName,
}

#[derive(Debug, Clone, PartialEq, Eq, RustcEncodable, RustcDecodable, Hash, Copy, HashStable)]
pub enum ParamName {
    /// Some user-given name like `T` or `'x`.
    Plain(Ident),

    /// Synthetic name generated when user elided a lifetime in an impl header.
    ///
    /// E.g., the lifetimes in cases like these:
    ///
    ///     impl Foo for &u32
    ///     impl Foo<'_> for u32
    ///
    /// in that case, we rewrite to
    ///
    ///     impl<'f> Foo for &'f u32
    ///     impl<'f> Foo<'f> for u32
    ///
    /// where `'f` is something like `Fresh(0)`. The indices are
    /// unique per impl, but not necessarily continuous.
    Fresh(usize),

    /// Indicates an illegal name was given and an error has been
    /// repored (so we should squelch other derived errors). Occurs
    /// when, e.g., `'_` is used in the wrong place.
    Error,
}

impl ParamName {
    pub fn ident(&self) -> Ident {
        match *self {
            ParamName::Plain(ident) => ident,
            ParamName::Error | ParamName::Fresh(_) => keywords::UnderscoreLifetime.ident(),
        }
    }

    pub fn modern(&self) -> ParamName {
        match *self {
            ParamName::Plain(ident) => ParamName::Plain(ident.modern()),
            param_name => param_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, RustcEncodable, RustcDecodable, Hash, Copy, HashStable)]
pub enum LifetimeName {
    /// User-given names or fresh (synthetic) names.
    Param(ParamName),

    /// User wrote nothing (e.g., the lifetime in `&u32`).
    Implicit,

    /// Indicates an error during lowering (usually `'_` in wrong place)
    /// that was already reported.
    Error,

    /// User wrote specifies `'_`.
    Underscore,

    /// User wrote `'static`.
    Static,
}

impl LifetimeName {
    pub fn ident(&self) -> Ident {
        match *self {
            LifetimeName::Implicit => keywords::Invalid.ident(),
            LifetimeName::Error => keywords::Invalid.ident(),
            LifetimeName::Underscore => keywords::UnderscoreLifetime.ident(),
            LifetimeName::Static => keywords::StaticLifetime.ident(),
            LifetimeName::Param(param_name) => param_name.ident(),
        }
    }

    pub fn is_elided(&self) -> bool {
        match self {
            LifetimeName::Implicit | LifetimeName::Underscore => true,

            // It might seem surprising that `Fresh(_)` counts as
            // *not* elided -- but this is because, as far as the code
            // in the compiler is concerned -- `Fresh(_)` variants act
            // equivalently to "some fresh name". They correspond to
            // early-bound regions on an impl, in other words.
            LifetimeName::Error | LifetimeName::Param(_) | LifetimeName::Static => false,
        }
    }

    fn is_static(&self) -> bool {
        self == &LifetimeName::Static
    }

    pub fn modern(&self) -> LifetimeName {
        match *self {
            LifetimeName::Param(param_name) => LifetimeName::Param(param_name.modern()),
            lifetime_name => lifetime_name,
        }
    }
}

impl fmt::Display for Lifetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.ident().fmt(f)
    }
}

impl fmt::Debug for Lifetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
               "lifetime({}: {})",
               self.hir_id,
               print::to_string(print::NO_ANN, |s| s.print_lifetime(self)))
    }
}

impl Lifetime {
    pub fn is_elided(&self) -> bool {
        self.name.is_elided()
    }

    pub fn is_static(&self) -> bool {
        self.name.is_static()
    }
}

/// A `Path` is essentially Rust's notion of a name; for instance,
/// `std::cmp::PartialEq`. It's represented as a sequence of identifiers,
/// along with a bunch of supporting information.
#[derive(Clone, RustcEncodable, RustcDecodable, HashStable)]
pub struct Path {
    pub span: Span,
    /// The resolution for the path.
    pub res: Res,
    /// The segments in the path: the things separated by `::`.
    pub segments: HirVec<PathSegment>,
}

impl Path {
    pub fn is_global(&self) -> bool {
        !self.segments.is_empty() && self.segments[0].ident.name == keywords::PathRoot.name()
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "path({})", self)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", print::to_string(print::NO_ANN, |s| s.print_path(self, false)))
    }
}

/// A segment of a path: an identifier, an optional lifetime, and a set of
/// types.
#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub struct PathSegment {
    /// The identifier portion of this path segment.
    #[stable_hasher(project(name))]
    pub ident: Ident,
    // `id` and `res` are optional. We currently only use these in save-analysis,
    // any path segments without these will not have save-analysis info and
    // therefore will not have 'jump to def' in IDEs, but otherwise will not be
    // affected. (In general, we don't bother to get the defs for synthesized
    // segments, only for segments which have come from the AST).
    pub hir_id: Option<HirId>,
    pub res: Option<Res>,

    /// Type/lifetime parameters attached to this path. They come in
    /// two flavors: `Path<A,B,C>` and `Path(A,B) -> C`. Note that
    /// this is more than just simple syntactic sugar; the use of
    /// parens affects the region binding rules, so we preserve the
    /// distinction.
    pub args: Option<P<GenericArgs>>,

    /// Whether to infer remaining type parameters, if any.
    /// This only applies to expression and pattern paths, and
    /// out of those only the segments with no type parameters
    /// to begin with, e.g., `Vec::new` is `<Vec<..>>::new::<..>`.
    pub infer_types: bool,
}

impl PathSegment {
    /// Converts an identifier to the corresponding segment.
    pub fn from_ident(ident: Ident) -> PathSegment {
        PathSegment {
            ident,
            hir_id: None,
            res: None,
            infer_types: true,
            args: None,
        }
    }

    pub fn new(
        ident: Ident,
        hir_id: Option<HirId>,
        res: Option<Res>,
        args: GenericArgs,
        infer_types: bool,
    ) -> Self {
        PathSegment {
            ident,
            hir_id,
            res,
            infer_types,
            args: if args.is_empty() {
                None
            } else {
                Some(P(args))
            }
        }
    }

    // FIXME: hack required because you can't create a static
    // `GenericArgs`, so you can't just return a `&GenericArgs`.
    pub fn with_generic_args<F, R>(&self, f: F) -> R
        where F: FnOnce(&GenericArgs) -> R
    {
        let dummy = GenericArgs::none();
        f(if let Some(ref args) = self.args {
            &args
        } else {
            &dummy
        })
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub struct ConstArg {
    pub value: AnonConst,
    pub span: Span,
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub enum GenericArg {
    Lifetime(Lifetime),
    Type(Ty),
    Const(ConstArg),
}

impl GenericArg {
    pub fn span(&self) -> Span {
        match self {
            GenericArg::Lifetime(l) => l.span,
            GenericArg::Type(t) => t.span,
            GenericArg::Const(c) => c.span,
        }
    }

    pub fn id(&self) -> HirId {
        match self {
            GenericArg::Lifetime(l) => l.hir_id,
            GenericArg::Type(t) => t.hir_id,
            GenericArg::Const(c) => c.value.hir_id,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub struct GenericArgs {
    /// The generic arguments for this path segment.
    pub args: HirVec<GenericArg>,
    /// Bindings (equality constraints) on associated types, if present.
    /// E.g., `Foo<A = Bar>`.
    pub bindings: HirVec<TypeBinding>,
    /// Were arguments written in parenthesized form `Fn(T) -> U`?
    /// This is required mostly for pretty-printing and diagnostics,
    /// but also for changing lifetime elision rules to be "function-like".
    pub parenthesized: bool,
}

impl GenericArgs {
    pub fn none() -> Self {
        Self {
            args: HirVec::new(),
            bindings: HirVec::new(),
            parenthesized: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.args.is_empty() && self.bindings.is_empty() && !self.parenthesized
    }

    pub fn inputs(&self) -> &[Ty] {
        if self.parenthesized {
            for arg in &self.args {
                match arg {
                    GenericArg::Lifetime(_) => {}
                    GenericArg::Type(ref ty) => {
                        if let TyKind::Tup(ref tys) = ty.node {
                            return tys;
                        }
                        break;
                    }
                    GenericArg::Const(_) => {}
                }
            }
        }
        bug!("GenericArgs::inputs: not a `Fn(T) -> U`");
    }

    pub fn own_counts(&self) -> GenericParamCount {
        // We could cache this as a property of `GenericParamCount`, but
        // the aim is to refactor this away entirely eventually and the
        // presence of this method will be a constant reminder.
        let mut own_counts: GenericParamCount = Default::default();

        for arg in &self.args {
            match arg {
                GenericArg::Lifetime(_) => own_counts.lifetimes += 1,
                GenericArg::Type(_) => own_counts.types += 1,
                GenericArg::Const(_) => own_counts.consts += 1,
            };
        }

        own_counts
    }
}

/// A modifier on a bound, currently this is only used for `?Sized`, where the
/// modifier is `Maybe`. Negative bounds should also be handled here.
#[derive(Copy, Clone, PartialEq, Eq, RustcEncodable, RustcDecodable, Hash, Debug, HashStable)]
pub enum TraitBoundModifier {
    None,
    Maybe,
}

/// The AST represents all type param bounds as types.
/// `typeck::collect::compute_bounds` matches these against
/// the "special" built-in traits (see `middle::lang_items`) and
/// detects `Copy`, `Send` and `Sync`.
#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub enum GenericBound {
    Trait(PolyTraitRef, TraitBoundModifier),
    Outlives(Lifetime),
}

impl GenericBound {
    pub fn span(&self) -> Span {
        match self {
            &GenericBound::Trait(ref t, ..) => t.span,
            &GenericBound::Outlives(ref l) => l.span,
        }
    }
}

pub type GenericBounds = HirVec<GenericBound>;

#[derive(Copy, Clone, PartialEq, Eq, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub enum LifetimeParamKind {
    // Indicates that the lifetime definition was explicitly declared (e.g., in
    // `fn foo<'a>(x: &'a u8) -> &'a u8 { x }`).
    Explicit,

    // Indicates that the lifetime definition was synthetically added
    // as a result of an in-band lifetime usage (e.g., in
    // `fn foo(x: &'a u8) -> &'a u8 { x }`).
    InBand,

    // Indication that the lifetime was elided (e.g., in both cases in
    // `fn foo(x: &u8) -> &'_ u8 { x }`).
    Elided,

    // Indication that the lifetime name was somehow in error.
    Error,
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub enum GenericParamKind {
    /// A lifetime definition (e.g., `'a: 'b + 'c + 'd`).
    Lifetime {
        kind: LifetimeParamKind,
    },
    Type {
        default: Option<P<Ty>>,
        synthetic: Option<SyntheticTyParamKind>,
    },
    Const {
        ty: P<Ty>,
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub struct GenericParam {
    pub hir_id: HirId,
    pub name: ParamName,
    pub attrs: HirVec<Attribute>,
    pub bounds: GenericBounds,
    pub span: Span,
    pub pure_wrt_drop: bool,

    pub kind: GenericParamKind,
}

#[derive(Default)]
pub struct GenericParamCount {
    pub lifetimes: usize,
    pub types: usize,
    pub consts: usize,
}

/// Represents lifetimes and type parameters attached to a declaration
/// of a function, enum, trait, etc.
#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub struct Generics {
    pub params: HirVec<GenericParam>,
    pub where_clause: WhereClause,
    pub span: Span,
}

impl Generics {
    pub fn empty() -> Generics {
        Generics {
            params: HirVec::new(),
            where_clause: WhereClause {
                hir_id: DUMMY_HIR_ID,
                predicates: HirVec::new(),
            },
            span: DUMMY_SP,
        }
    }

    pub fn own_counts(&self) -> GenericParamCount {
        // We could cache this as a property of `GenericParamCount`, but
        // the aim is to refactor this away entirely eventually and the
        // presence of this method will be a constant reminder.
        let mut own_counts: GenericParamCount = Default::default();

        for param in &self.params {
            match param.kind {
                GenericParamKind::Lifetime { .. } => own_counts.lifetimes += 1,
                GenericParamKind::Type { .. } => own_counts.types += 1,
                GenericParamKind::Const { .. } => own_counts.consts += 1,
            };
        }

        own_counts
    }

    pub fn get_named(&self, name: &InternedString) -> Option<&GenericParam> {
        for param in &self.params {
            if *name == param.name.ident().as_interned_str() {
                return Some(param);
            }
        }
        None
    }
}

/// Synthetic type parameters are converted to another form during lowering; this allows
/// us to track the original form they had, and is useful for error messages.
#[derive(Copy, Clone, PartialEq, Eq, RustcEncodable, RustcDecodable, Hash, Debug, HashStable)]
pub enum SyntheticTyParamKind {
    ImplTrait
}

/// A where-clause in a definition.
#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub struct WhereClause {
    pub hir_id: HirId,
    pub predicates: HirVec<WherePredicate>,
}

impl WhereClause {
    pub fn span(&self) -> Option<Span> {
        self.predicates.iter().map(|predicate| predicate.span())
            .fold(None, |acc, i| match (acc, i) {
                (None, i) => Some(i),
                (Some(acc), i) => {
                    Some(acc.to(i))
                }
            })
    }
}

/// A single predicate in a where-clause.
#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub enum WherePredicate {
    /// A type binding (e.g., `for<'c> Foo: Send + Clone + 'c`).
    BoundPredicate(WhereBoundPredicate),
    /// A lifetime predicate (e.g., `'a: 'b + 'c`).
    RegionPredicate(WhereRegionPredicate),
    /// An equality predicate (unsupported).
    EqPredicate(WhereEqPredicate),
}

impl WherePredicate {
    pub fn span(&self) -> Span {
        match self {
            &WherePredicate::BoundPredicate(ref p) => p.span,
            &WherePredicate::RegionPredicate(ref p) => p.span,
            &WherePredicate::EqPredicate(ref p) => p.span,
        }
    }
}

/// A type bound (e.g., `for<'c> Foo: Send + Clone + 'c`).
#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub struct WhereBoundPredicate {
    pub span: Span,
    /// Any generics from a `for` binding.
    pub bound_generic_params: HirVec<GenericParam>,
    /// The type being bounded.
    pub bounded_ty: P<Ty>,
    /// Trait and lifetime bounds (e.g., `Clone + Send + 'static`).
    pub bounds: GenericBounds,
}

/// A lifetime predicate (e.g., `'a: 'b + 'c`).
#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub struct WhereRegionPredicate {
    pub span: Span,
    pub lifetime: Lifetime,
    pub bounds: GenericBounds,
}

/// An equality predicate (e.g., `T = int`); currently unsupported.
#[derive(Clone, RustcEncodable, RustcDecodable, Debug, HashStable)]
pub struct WhereEqPredicate {
    pub hir_id: HirId,
    pub span: Span,
    pub lhs_ty: P<Ty>,
    pub rhs_ty: P<Ty>,
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct ModuleItems {
    // Use BTreeSets here so items are in the same order as in the
    // list of all items in Crate
    pub items: BTreeSet<HirId>,
    pub trait_items: BTreeSet<TraitItemId>,
    pub impl_items: BTreeSet<ImplItemId>,
}

/// The top-level data structure that stores the entire contents of
/// the crate currently being compiled.
///
/// For more details, see the [rustc guide].
///
/// [rustc guide]: https://rust-lang.github.io/rustc-guide/hir.html
#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Crate {
    pub module: Mod,
    pub attrs: HirVec<Attribute>,
    pub span: Span,
    pub exported_macros: HirVec<MacroDef>,

    // N.B., we use a BTreeMap here so that `visit_all_items` iterates
    // over the ids in increasing order. In principle it should not
    // matter what order we visit things in, but in *practice* it
    // does, because it can affect the order in which errors are
    // detected, which in turn can make compile-fail tests yield
    // slightly different results.
    pub items: BTreeMap<HirId, Item>,

    pub trait_items: BTreeMap<TraitItemId, TraitItem>,
    pub impl_items: BTreeMap<ImplItemId, ImplItem>,
    pub bodies: BTreeMap<BodyId, Body>,
    pub trait_impls: BTreeMap<DefId, Vec<HirId>>,

    /// A list of the body ids written out in the order in which they
    /// appear in the crate. If you're going to process all the bodies
    /// in the crate, you should iterate over this list rather than the keys
    /// of bodies.
    pub body_ids: Vec<BodyId>,

    /// A list of modules written out in the order in which they
    /// appear in the crate. This includes the main crate module.
    pub modules: BTreeMap<NodeId, ModuleItems>,
}

impl Crate {
    pub fn item(&self, id: HirId) -> &Item {
        &self.items[&id]
    }

    pub fn trait_item(&self, id: TraitItemId) -> &TraitItem {
        &self.trait_items[&id]
    }

    pub fn impl_item(&self, id: ImplItemId) -> &ImplItem {
        &self.impl_items[&id]
    }

    /// Visits all items in the crate in some deterministic (but
    /// unspecified) order. If you just need to process every item,
    /// but don't care about nesting, this method is the best choice.
    ///
    /// If you do care about nesting -- usually because your algorithm
    /// follows lexical scoping rules -- then you want a different
    /// approach. You should override `visit_nested_item` in your
    /// visitor and then call `intravisit::walk_crate` instead.
    pub fn visit_all_item_likes<'hir, V>(&'hir self, visitor: &mut V)
        where V: itemlikevisit::ItemLikeVisitor<'hir>
    {
        for (_, item) in &self.items {
            visitor.visit_item(item);
        }

        for (_, trait_item) in &self.trait_items {
            visitor.visit_trait_item(trait_item);
        }

        for (_, impl_item) in &self.impl_items {
            visitor.visit_impl_item(impl_item);
        }
    }

    /// A parallel version of `visit_all_item_likes`.
    pub fn par_visit_all_item_likes<'hir, V>(&'hir self, visitor: &V)
        where V: itemlikevisit::ParItemLikeVisitor<'hir> + Sync + Send
    {
        parallel!({
            par_for_each_in(&self.items, |(_, item)| {
                visitor.visit_item(item);
            });
        }, {
            par_for_each_in(&self.trait_items, |(_, trait_item)| {
                visitor.visit_trait_item(trait_item);
            });
        }, {
            par_for_each_in(&self.impl_items, |(_, impl_item)| {
                visitor.visit_impl_item(impl_item);
            });
        });
    }
