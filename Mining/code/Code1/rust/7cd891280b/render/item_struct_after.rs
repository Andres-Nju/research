fn item_struct(w: &mut fmt::Formatter, cx: &Context, it: &clean::Item,
               s: &clean::Struct) -> fmt::Result {
    write!(w, "<pre class='rust struct'>")?;
    render_attributes(w, it)?;
    render_struct(w,
                  it,
                  Some(&s.generics),
                  s.struct_type,
                  &s.fields,
                  "",
                  true)?;
    write!(w, "</pre>")?;

    document(w, cx, it)?;
    let mut fields = s.fields.iter().filter_map(|f| {
        match f.inner {
            clean::StructFieldItem(ref ty) => Some((f, ty)),
            _ => None,
        }
    }).peekable();
    if let doctree::Plain = s.struct_type {
        if fields.peek().is_some() {
            write!(w, "<h2 class='fields'>Fields</h2>")?;
            for (field, ty) in fields {
                write!(w, "<span id='{shortty}.{name}' class='{shortty}'><code>{name}: {ty}</code>
                           </span><span class='stab {stab}'></span>",
                       shortty = ItemType::StructField,
                       stab = field.stability_class(),
                       name = field.name.as_ref().unwrap(),
                       ty = ty)?;
                document(w, cx, field)?;
            }
        }
    }
    render_assoc_items(w, cx, it, it.def_id, AssocItemRender::All)
}
