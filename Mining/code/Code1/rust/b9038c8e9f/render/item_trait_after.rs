fn item_trait(w: &mut fmt::Formatter, cx: &Context, it: &clean::Item,
              t: &clean::Trait) -> fmt::Result {
    let mut bounds = String::new();
    let mut bounds_plain = String::new();
    if !t.bounds.is_empty() {
        if !bounds.is_empty() {
            bounds.push(' ');
            bounds_plain.push(' ');
        }
        bounds.push_str(": ");
        bounds_plain.push_str(": ");
        for (i, p) in t.bounds.iter().enumerate() {
            if i > 0 {
                bounds.push_str(" + ");
                bounds_plain.push_str(" + ");
            }
            bounds.push_str(&format!("{}", *p));
            bounds_plain.push_str(&format!("{:#}", *p));
        }
    }

    // Output the trait definition
    write!(w, "<pre class='rust trait'>{}{}trait {}{}{}{} ",
           VisSpace(&it.visibility),
           UnsafetySpace(t.unsafety),
           it.name.as_ref().unwrap(),
           t.generics,
           bounds,
           // Where clauses in traits are indented nine spaces, per rustdoc.css
           WhereClause(&t.generics, 9))?;

    let types = t.items.iter().filter(|m| m.is_associated_type()).collect::<Vec<_>>();
    let consts = t.items.iter().filter(|m| m.is_associated_const()).collect::<Vec<_>>();
    let required = t.items.iter().filter(|m| m.is_ty_method()).collect::<Vec<_>>();
    let provided = t.items.iter().filter(|m| m.is_method()).collect::<Vec<_>>();

    if t.items.is_empty() {
        write!(w, "{{ }}")?;
    } else {
        // FIXME: we should be using a derived_id for the Anchors here
        write!(w, "{{\n")?;
        for t in &types {
            write!(w, "    ")?;
            render_assoc_item(w, t, AssocItemLink::Anchor(None), ItemType::Trait)?;
            write!(w, ";\n")?;
        }
        if !types.is_empty() && !consts.is_empty() {
            w.write_str("\n")?;
        }
        for t in &consts {
            write!(w, "    ")?;
            render_assoc_item(w, t, AssocItemLink::Anchor(None), ItemType::Trait)?;
            write!(w, ";\n")?;
        }
        if !consts.is_empty() && !required.is_empty() {
            w.write_str("\n")?;
        }
        for m in &required {
            write!(w, "    ")?;
            render_assoc_item(w, m, AssocItemLink::Anchor(None), ItemType::Trait)?;
            write!(w, ";\n")?;
        }
        if !required.is_empty() && !provided.is_empty() {
            w.write_str("\n")?;
        }
        for m in &provided {
            write!(w, "    ")?;
            render_assoc_item(w, m, AssocItemLink::Anchor(None), ItemType::Trait)?;
            write!(w, " {{ ... }}\n")?;
        }
        write!(w, "}}")?;
    }
    write!(w, "</pre>")?;

    // Trait documentation
    document(w, cx, it)?;

    fn trait_item(w: &mut fmt::Formatter, cx: &Context, m: &clean::Item, t: &clean::Item)
                  -> fmt::Result {
        let name = m.name.as_ref().unwrap();
        let item_type = m.type_();
        let id = derive_id(format!("{}.{}", item_type, name));
        let ns_id = derive_id(format!("{}.{}", name, item_type.name_space()));
        write!(w, "<h3 id='{id}' class='method'>\
                   <span id='{ns_id}' class='invisible'><code>",
               id = id,
               ns_id = ns_id)?;
        render_assoc_item(w, m, AssocItemLink::Anchor(Some(&id)), ItemType::Impl)?;
        write!(w, "</code>")?;
        render_stability_since(w, m, t)?;
        write!(w, "</span></h3>")?;
        document(w, cx, m)?;
        Ok(())
    }

    if !types.is_empty() {
        write!(w, "
            <h2 id='associated-types'>Associated Types</h2>
            <div class='methods'>
        ")?;
        for t in &types {
            trait_item(w, cx, *t, it)?;
        }
        write!(w, "</div>")?;
    }

    if !consts.is_empty() {
        write!(w, "
            <h2 id='associated-const'>Associated Constants</h2>
            <div class='methods'>
        ")?;
        for t in &consts {
            trait_item(w, cx, *t, it)?;
        }
        write!(w, "</div>")?;
    }

    // Output the documentation for each function individually
    if !required.is_empty() {
        write!(w, "
            <h2 id='required-methods'>Required Methods</h2>
            <div class='methods'>
        ")?;
        for m in &required {
            trait_item(w, cx, *m, it)?;
        }
        write!(w, "</div>")?;
    }
    if !provided.is_empty() {
        write!(w, "
            <h2 id='provided-methods'>Provided Methods</h2>
            <div class='methods'>
        ")?;
        for m in &provided {
            trait_item(w, cx, *m, it)?;
        }
        write!(w, "</div>")?;
    }

    // If there are methods directly on this trait object, render them here.
    render_assoc_items(w, cx, it, it.def_id, AssocItemRender::All)?;

    let cache = cache();
    write!(w, "
        <h2 id='implementors'>Implementors</h2>
        <ul class='item-list' id='implementors-list'>
    ")?;
    if let Some(implementors) = cache.implementors.get(&it.def_id) {
        for i in implementors {
            write!(w, "<li><code>")?;
            fmt_impl_for_trait_page(&i.impl_, w)?;
            writeln!(w, "</code></li>")?;
        }
    }
    write!(w, "</ul>")?;
    write!(w, r#"<script type="text/javascript" async
                         src="{root_path}/implementors/{path}/{ty}.{name}.js">
                 </script>"#,
           root_path = vec![".."; cx.current.len()].join("/"),
           path = if it.def_id.is_local() {
               cx.current.join("/")
           } else {
               let (ref path, _) = cache.external_paths[&it.def_id];
               path[..path.len() - 1].join("/")
           },
           ty = it.type_().css_class(),
           name = *it.name.as_ref().unwrap())?;
    Ok(())
}
