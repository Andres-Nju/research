fn short_stability(item: &clean::Item, cx: &Context, show_reason: bool) -> Vec<String> {
    let mut stability = vec![];

    if let Some(stab) = item.stability.as_ref() {
        let deprecated_reason = if show_reason && !stab.deprecated_reason.is_empty() {
            format!(": {}", stab.deprecated_reason)
        } else {
            String::new()
        };
        if !stab.deprecated_since.is_empty() {
            let since = if show_reason {
                format!(" since {}", Escape(&stab.deprecated_since))
            } else {
                String::new()
            };
            let text = format!("Deprecated{}{}", since, MarkdownHtml(&deprecated_reason));
            stability.push(format!("<div class='stab deprecated'>{}</div>", text))
        };

        if stab.level == stability::Unstable {
            if show_reason {
                let unstable_extra = match (!stab.feature.is_empty(),
                                            &cx.shared.issue_tracker_base_url,
                                            stab.issue) {
                    (true, &Some(ref tracker_url), Some(issue_no)) if issue_no > 0 =>
                        format!(" (<code>{}</code> <a href=\"{}{}\">#{}</a>)",
                                Escape(&stab.feature), tracker_url, issue_no, issue_no),
                    (false, &Some(ref tracker_url), Some(issue_no)) if issue_no > 0 =>
                        format!(" (<a href=\"{}{}\">#{}</a>)", Escape(&tracker_url), issue_no,
                                issue_no),
                    (true, ..) =>
                        format!(" (<code>{}</code>)", Escape(&stab.feature)),
                    _ => String::new(),
                };
                if stab.unstable_reason.is_empty() {
                    stability.push(format!("<div class='stab unstable'>\
                                            <span class=microscope>🔬</span> \
                                            This is a nightly-only experimental API. {}</div>",
                                   unstable_extra));
                } else {
                    let text = format!("<summary><span class=microscope>🔬</span> \
                                        This is a nightly-only experimental API. {}</summary>{}",
                                       unstable_extra, MarkdownHtml(&stab.unstable_reason));
                    stability.push(format!("<div class='stab unstable'><details>{}</details></div>",
                                   text));
                }
            } else {
                stability.push(format!("<div class='stab unstable'>Experimental</div>"))
            }
        };
    } else if let Some(depr) = item.deprecation.as_ref() {
        let note = if show_reason && !depr.note.is_empty() {
            format!(": {}", depr.note)
        } else {
            String::new()
        };
        let since = if show_reason && !depr.since.is_empty() {
            format!(" since {}", Escape(&depr.since))
        } else {
            String::new()
        };

        let text = format!("Deprecated{}{}", since, MarkdownHtml(&note));
        stability.push(format!("<div class='stab deprecated'>{}</div>", text))
    }

    stability
}
