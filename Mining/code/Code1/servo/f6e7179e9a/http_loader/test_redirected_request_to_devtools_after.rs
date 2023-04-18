fn test_redirected_request_to_devtools() {
    let post_handler = move |request: HyperRequest, response: HyperResponse| {
        assert_eq!(request.method, Method::Get);
        response.send(b"Yay!").unwrap();
    };
    let (mut post_server, post_url) = make_server(post_handler);

    let post_redirect_url = post_url.clone();
    let pre_handler = move |request: HyperRequest, mut response: HyperResponse| {
        assert_eq!(request.method, Method::Post);
        response.headers_mut().set(Location(post_redirect_url.to_string()));
        *response.status_mut() = StatusCode::MovedPermanently;
        response.send(b"").unwrap();
    };
    let (mut pre_server, pre_url) = make_server(pre_handler);

    let request = Request::from_init(RequestInit {
        url: pre_url.clone(),
        method: Method::Post,
        destination: Destination::Document,
        origin: pre_url.clone(),
        pipeline_id: Some(TEST_PIPELINE_ID),
        .. RequestInit::default()
    });
    let (devtools_chan, devtools_port) = mpsc::channel();
    fetch(request, Some(devtools_chan));

    let _ = pre_server.close();
    let _ = post_server.close();

    let devhttprequest = expect_devtools_http_request(&devtools_port);
    let devhttpresponse = expect_devtools_http_response(&devtools_port);

    assert!(devhttprequest.method == Method::Post);
    assert!(devhttprequest.url == pre_url);
    assert!(devhttpresponse.status == Some((301, b"Moved Permanently".to_vec())));

    let devhttprequest = expect_devtools_http_request(&devtools_port);
    let devhttpresponse = expect_devtools_http_response(&devtools_port);

    assert!(devhttprequest.method == Method::Get);
    assert!(devhttprequest.url == post_url);
    assert!(devhttpresponse.status == Some((200, b"OK".to_vec())));
}
