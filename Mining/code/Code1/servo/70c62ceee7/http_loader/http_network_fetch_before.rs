fn http_network_fetch(
    request: &Request,
    credentials_flag: bool,
    done_chan: &mut DoneChannel,
    context: &FetchContext,
) -> Response {
    let mut response_end_timer = ResponseEndTimer(Some(context.timing.clone()));

    // Step 1
    // nothing to do here, since credentials_flag is already a boolean

    // Step 2
    // TODO be able to create connection using current url's origin and credentials

    // Step 3
    // TODO be able to tell if the connection is a failure

    // Step 4
    // TODO: check whether the connection is HTTP/2

    // Step 5
    let url = request.current_url();

    let request_id = context
        .devtools_chan
        .as_ref()
        .map(|_| uuid::Uuid::new_v4().to_simple().to_string());

    if log_enabled!(log::Level::Info) {
        info!("request for {} ({:?})", url, request.method);
        for header in request.headers.iter() {
            info!(" - {:?}", header);
        }
    }

    // XHR uses the default destination; other kinds of fetches (which haven't been implemented yet)
    // do not. Once we support other kinds of fetches we'll need to be more fine grained here
    // since things like image fetches are classified differently by devtools
    let is_xhr = request.destination == Destination::None;
    let response_future = obtain_response(
        &context.state.client,
        &url,
        &request.method,
        &request.headers,
        &request.body,
        &request.method,
        &request.pipeline_id,
        request.redirect_count + 1,
        request_id.as_ref().map(Deref::deref),
        is_xhr,
        context,
    );

    let pipeline_id = request.pipeline_id;
    // This will only get the headers, the body is read later
    let (res, msg) = match response_future.wait() {
        Ok(wrapped_response) => wrapped_response,
        Err(error) => return Response::network_error(error),
    };

    if log_enabled!(log::Level::Info) {
        info!("response for {}", url);
        for header in res.headers().iter() {
            info!(" - {:?}", header);
        }
    }

    let header_strings: Vec<&str> = res
        .headers()
        .get_all("Timing-Allow-Origin")
        .iter()
        .map(|header_value| header_value.to_str().unwrap_or(""))
        .collect();
    let wildcard_present = header_strings.iter().any(|header_str| *header_str == "*");
    // The spec: https://www.w3.org/TR/resource-timing-2/#sec-timing-allow-origin
    // says that a header string is either an origin or a wildcard so we can just do a straight
    // check against the document origin
    let req_origin_in_timing_allow = header_strings
        .iter()
        .any(|header_str| match request.origin {
            SpecificOrigin(ref immutable_request_origin) => {
                *header_str == immutable_request_origin.ascii_serialization()
            },
            _ => false,
        });

    let is_same_origin = request.url_list.iter().any(|url| match request.origin {
        SpecificOrigin(ref immutable_request_origin) => {
            url.clone().into_url().origin().ascii_serialization() ==
                immutable_request_origin.ascii_serialization()
        },
        _ => false,
    });

    if !(is_same_origin || req_origin_in_timing_allow || wildcard_present) {
        context.timing.lock().unwrap().mark_timing_check_failed();
    }

    let timing = context.timing.lock().unwrap().clone();
    let mut response = Response::new(url.clone(), timing);

    response.status = Some((
        res.status(),
        res.status().canonical_reason().unwrap_or("").into(),
    ));
    debug!("got {:?} response for {:?}", res.status(), request.url());
    response.raw_status = Some((
        res.status().as_u16(),
        res.status().canonical_reason().unwrap_or("").into(),
    ));
    response.headers = res.headers().clone();
    response.referrer = request.referrer.to_url().cloned();
    response.referrer_policy = request.referrer_policy.clone();

    let res_body = response.body.clone();

    // We're about to spawn a future to be waited on here
    let (done_sender, done_receiver) = unbounded();
    *done_chan = Some((done_sender.clone(), done_receiver));
    let meta = match response
        .metadata()
        .expect("Response metadata should exist at this stage")
    {
        FetchMetadata::Unfiltered(m) => m,
        FetchMetadata::Filtered { unsafe_, .. } => unsafe_,
    };

    let devtools_sender = context.devtools_chan.clone();
    let meta_status = meta.status;
    let meta_headers = meta.headers;
    let cancellation_listener = context.cancellation_listener.clone();
    if cancellation_listener.lock().unwrap().cancelled() {
        return Response::network_error(NetworkError::Internal("Fetch aborted".into()));
    }

    *res_body.lock().unwrap() = ResponseBody::Receiving(vec![]);
    let res_body2 = res_body.clone();

    if let Some(ref sender) = devtools_sender {
        if let Some(m) = msg {
            send_request_to_devtools(m, &sender);
        }

        // --- Tell devtools that we got a response
        // Send an HttpResponse message to devtools with the corresponding request_id
        if let Some(pipeline_id) = pipeline_id {
            send_response_to_devtools(
                &sender,
                request_id.unwrap(),
                meta_headers.map(Serde::into_inner),
                meta_status,
                pipeline_id,
            );
        }
    }

    let done_sender2 = done_sender.clone();
    let done_sender3 = done_sender.clone();
    let timing_ptr2 = context.timing.clone();
    let timing_ptr3 = context.timing.clone();
    let url1 = request.url();
    let url2 = url1.clone();
    HANDLE.lock().unwrap().spawn(
        res.into_body()
            .map_err(|_| ())
            .fold(res_body, move |res_body, chunk| {
                if cancellation_listener.lock().unwrap().cancelled() {
                    *res_body.lock().unwrap() = ResponseBody::Done(vec![]);
                    let _ = done_sender.send(Data::Cancelled);
                    return future::failed(());
                }
                if let ResponseBody::Receiving(ref mut body) = *res_body.lock().unwrap() {
                    let bytes = chunk.into_bytes();
                    body.extend_from_slice(&*bytes);
                    let _ = done_sender.send(Data::Payload(bytes.to_vec()));
                }
                future::ok(res_body)
            })
            .and_then(move |res_body| {
                debug!("successfully finished response for {:?}", url1);
                let mut body = res_body.lock().unwrap();
                let completed_body = match *body {
                    ResponseBody::Receiving(ref mut body) => mem::replace(body, vec![]),
                    _ => vec![],
                };
                *body = ResponseBody::Done(completed_body);
                timing_ptr2
                    .lock()
                    .unwrap()
                    .set_attribute(ResourceAttribute::ResponseEnd);
                let _ = done_sender2.send(Data::Done);
                future::ok(())
            })
            .map_err(move |_| {
                debug!("finished response for {:?} with error", url2);
                let mut body = res_body2.lock().unwrap();
                let completed_body = match *body {
                    ResponseBody::Receiving(ref mut body) => mem::replace(body, vec![]),
                    _ => vec![],
                };
                *body = ResponseBody::Done(completed_body);
                timing_ptr3
                    .lock()
                    .unwrap()
                    .set_attribute(ResourceAttribute::ResponseEnd);
                let _ = done_sender3.send(Data::Done);
            }),
    );

    // TODO these substeps aren't possible yet
    // Substep 1

    // Substep 2

    // TODO Determine if response was retrieved over HTTPS
    // TODO Servo needs to decide what ciphers are to be treated as "deprecated"
    response.https_state = HttpsState::None;

    // TODO Read request

    // Step 6-11
    // (needs stream bodies)

    // Step 12
    // TODO when https://bugzilla.mozilla.org/show_bug.cgi?id=1030660
    // is resolved, this step will become uneccesary
    // TODO this step
    if let Some(encoding) = response.headers.typed_get::<ContentEncoding>() {
        if encoding.contains("gzip") {
        } else if encoding.contains("compress") {
        }
    };

    // Step 13
    // TODO this step isn't possible yet (CSP)

    // Step 14, update the cached response, done via the shared response body.

    // TODO this step isn't possible yet
    // Step 15
    if credentials_flag {
        set_cookies_from_headers(&url, &response.headers, &context.state.cookie_jar);
    }

    // TODO these steps
    // Step 16
    // Substep 1
    // Substep 2
    // Sub-substep 1
    // Sub-substep 2
    // Sub-substep 3
    // Sub-substep 4
    // Substep 3

    // Step 16

    // Ensure we don't override "responseEnd" on successful return of this function
    response_end_timer.neuter();

    response
}
