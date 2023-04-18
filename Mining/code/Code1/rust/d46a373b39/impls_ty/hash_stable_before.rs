    fn hash_stable<W: StableHasherResult>(
        &self,
        hcx: &mut StableHashingContext<'a>,
        hasher: &mut StableHasher<W>,
    ) {
        ty::tls::with_opt(|tcx| {
            trace!("hashing {:?}", *self);
            let tcx = tcx.expect("can't hash AllocIds during hir lowering");
            let alloc_kind = tcx.alloc_map.lock().get(*self);
            alloc_kind.hash_stable(hcx, hasher);
        });
    }
}

// Allocations treat their relocations specially
impl<'a> HashStable<StableHashingContext<'a>> for mir::interpret::Allocation {
    fn hash_stable<W: StableHasherResult>(
        &self,
        hcx: &mut StableHashingContext<'a>,
        hasher: &mut StableHasher<W>,
    ) {
        let mir::interpret::Allocation {
            bytes, relocations, undef_mask, align, mutability,
            extra: _,
        } = self;
        bytes.hash_stable(hcx, hasher);
        for reloc in relocations.iter() {
            reloc.hash_stable(hcx, hasher);
        }
        undef_mask.hash_stable(hcx, hasher);
        align.hash_stable(hcx, hasher);
        mutability.hash_stable(hcx, hasher);
    }
}

impl_stable_hash_for!(enum ::syntax::ast::Mutability {
    Immutable,
    Mutable
});

impl<'a> ToStableHashKey<StableHashingContext<'a>> for region::Scope {
    type KeyType = region::Scope;

    #[inline]
    fn to_stable_hash_key(&self, _: &StableHashingContext<'a>) -> region::Scope {
        *self
    }
}

impl<'a> HashStable<StableHashingContext<'a>> for ty::TyVid {
    fn hash_stable<W: StableHasherResult>(&self,
                                          _hcx: &mut StableHashingContext<'a>,
                                          _hasher: &mut StableHasher<W>) {
        // TyVid values are confined to an inference context and hence
        // should not be hashed.
        bug!("ty::TyKind::hash_stable() - can't hash a TyVid {:?}.", *self)
    }
}

impl<'a> HashStable<StableHashingContext<'a>> for ty::IntVid {
    fn hash_stable<W: StableHasherResult>(&self,
                                          _hcx: &mut StableHashingContext<'a>,
                                          _hasher: &mut StableHasher<W>) {
        // IntVid values are confined to an inference context and hence
        // should not be hashed.
        bug!("ty::TyKind::hash_stable() - can't hash an IntVid {:?}.", *self)
    }
