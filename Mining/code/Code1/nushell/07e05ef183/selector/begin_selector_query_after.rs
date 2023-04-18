pub fn begin_selector_query(input_html: String, selector: &Selector) -> Vec<Value> {
    if !selector.as_table.value.is_string() {
        retrieve_tables(input_html.as_str(), &selector.as_table, selector.inspect)
    } else {
        match selector.attribute.is_empty() {
            true => execute_selector_query(
                input_html.as_str(),
                selector.query.as_str(),
                selector.as_html,
            ),
            false => execute_selector_query_with_attribute(
                input_html.as_str(),
                selector.query.as_str(),
                selector.attribute.as_str(),
            ),
        }
    }
}
