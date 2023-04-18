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
