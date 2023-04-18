fn router_works() {
    yew::start_app_in_element::<Root>(gloo_utils::document().get_element_by_id("output").unwrap());

    assert_eq!("Home", obtain_result_by_id("result"));

    let initial_length = history_length();

    click("button"); // replacing the current route
    assert_eq!("2", obtain_result_by_id("result-params"));
    assert_eq!("bar", obtain_result_by_id("result-query"));
    assert_eq!(initial_length, history_length());

    click("button"); // pushing a new route
    assert_eq!("3", obtain_result_by_id("result-params"));
    assert_eq!("baz", obtain_result_by_id("result-query"));
    assert_eq!(initial_length + 1, history_length());
}
