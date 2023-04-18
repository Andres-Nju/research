pub(super) fn with_outer_attributes(
    p: &mut Parser,
    f: impl Fn(&mut Parser) -> Option<CompletedMarker>,
) -> bool {
    let am = p.start();
    let has_attrs = p.at(T![#]);
    attributes::outer_attributes(p);    
    let cm = f(p);
    let success = cm.is_some();

    match (has_attrs, cm) {
        (true, Some(cm)) => {
            let kind = cm.kind();
            cm.undo_completion(p).abandon(p);
            am.complete(p, kind);
        }
        _ => am.abandon(p),
    }

    success
}
