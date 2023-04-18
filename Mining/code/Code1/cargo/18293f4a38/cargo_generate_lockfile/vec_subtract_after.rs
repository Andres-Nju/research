        fn vec_subtract<'a>(a: &[&'a PackageId],
                            b: &[&'a PackageId]) -> Vec<&'a PackageId> {
            a.iter().filter(|a| {
                // If this package id is not found in `b`, then it's definitely
                // in the subtracted set
                let i = match b.binary_search(a) {
                    Ok(i) => i,
                    Err(..) => return true,
                };

                // If we've found `a` in `b`, then we iterate over all instances
                // (we know `b` is sorted) and see if they all have different
                // precise versions. If so, then `a` isn't actually in `b` so
                // we'll let it through.
                //
                // Note that we only check this for non-registry sources,
                // however, as registries contain enough version information in
                // the package id to disambiguate
                if a.source_id().is_registry() {
                    return false
                }
                b[i..].iter().take_while(|b| a == b).all(|b| {
                    a.source_id().precise() != b.source_id().precise()
                })
            }).cloned().collect()
        }

        // Map (package name, package source) to (removed versions, added versions).
        let mut changes = BTreeMap::new();
        let empty = (Vec::new(), Vec::new());
        for dep in previous_resolve.iter() {
            changes.entry(key(dep)).or_insert(empty.clone()).0.push(dep);
        }
        for dep in resolve.iter() {
            changes.entry(key(dep)).or_insert(empty.clone()).1.push(dep);
        }

        for (_, v) in changes.iter_mut() {
            let (ref mut old, ref mut new) = *v;
            old.sort();
            new.sort();
            let removed = vec_subtract(old, new);
            let added = vec_subtract(new, old);
            *old = removed;
            *new = added;
        }
        debug!("{:#?}", changes);

        changes.into_iter().map(|(_, v)| v).collect()
    }
