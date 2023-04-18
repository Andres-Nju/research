    fn shared_context(&self) -> &SharedStyleContext;

    /// Whether we're performing a parallel traversal.
    ///
    /// NB: We do this check on runtime. We could guarantee correctness in this
    /// regard via the type system via a `TraversalDriver` trait for this trait,
    /// that could be one of two concrete types. It's not clear whether the
    /// potential code size impact of that is worth it.
    fn is_parallel(&self) -> bool;
}

/// Manually resolve style by sequentially walking up the parent chain to the
/// first styled Element, ignoring pending restyles. The resolved style is made
/// available via a callback, and can be dropped by the time this function
/// returns in the display:none subtree case.
pub fn resolve_style<E>(
    context: &mut StyleContext<E>,
    element: E,
    rule_inclusion: RuleInclusion,
) -> ElementStyles
where
    E: TElement,
{
    use style_resolver::StyleResolverForElement;

    debug_assert!(rule_inclusion == RuleInclusion::DefaultOnly ||
                  element.borrow_data().map_or(true, |d| !d.has_styles()),
                  "Why are we here?");
    let mut ancestors_requiring_style_resolution = SmallVec::<[E; 16]>::new();

    // Clear the bloom filter, just in case the caller is reusing TLS.
    context.thread_local.bloom_filter.clear();

    let mut style = None;
    let mut ancestor = element.traversal_parent();
    while let Some(current) = ancestor {
        if rule_inclusion == RuleInclusion::All {
            if let Some(data) = current.borrow_data() {
                if let Some(ancestor_style) = data.styles.get_primary() {
                    style = Some(ancestor_style.clone());
                    break;
                }
            }
        }
        ancestors_requiring_style_resolution.push(current);
        ancestor = current.traversal_parent();
    }

    if let Some(ancestor) = ancestor {
        context.thread_local.bloom_filter.rebuild(ancestor);
        context.thread_local.bloom_filter.push(ancestor);
    }

    let mut layout_parent_style = style.clone();
    while let Some(style) = layout_parent_style.take() {
        if !style.is_display_contents() {
            layout_parent_style = Some(style);
            break;
        }

        ancestor = ancestor.unwrap().traversal_parent();
        layout_parent_style = ancestor.map(|a| {
            a.borrow_data().unwrap().styles.primary().clone()
        });
    }

    for ancestor in ancestors_requiring_style_resolution.iter().rev() {
        context.thread_local.bloom_filter.assert_complete(*ancestor);

        let primary_style =
            StyleResolverForElement::new(*ancestor, context, rule_inclusion)
                .resolve_primary_style(
                    style.as_ref().map(|s| &**s),
                    layout_parent_style.as_ref().map(|s| &**s)
                );

        let is_display_contents = primary_style.style.is_display_contents();

        style = Some(primary_style.style);
        if !is_display_contents {
            layout_parent_style = style.clone();
        }

        context.thread_local.bloom_filter.push(*ancestor);
    }

    context.thread_local.bloom_filter.assert_complete(element);
    StyleResolverForElement::new(element, context, rule_inclusion)
        .resolve_style(
            style.as_ref().map(|s| &**s),
            layout_parent_style.as_ref().map(|s| &**s)
        )
}
