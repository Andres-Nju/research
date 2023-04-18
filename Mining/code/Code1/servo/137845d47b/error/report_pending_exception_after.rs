pub unsafe fn report_pending_exception(cx: *mut JSContext, dispatch_event: bool) {
    if JS_IsExceptionPending(cx) {
        rooted!(in(cx) let mut value = UndefinedValue());
        if !JS_GetPendingException(cx, value.handle_mut()) {
            JS_ClearPendingException(cx);
            error!("Uncaught exception: JS_GetPendingException failed");
            return;
        }

        JS_ClearPendingException(cx);
        let error_info = if value.is_object() {
            rooted!(in(cx) let object = value.to_object());
            let error_info = ErrorInfo::from_native_error(cx, object.handle())
                .or_else(|| ErrorInfo::from_dom_exception(object.handle()));
            match error_info {
                Some(error_info) => error_info,
                None => {
                    error!("Uncaught exception: failed to extract information");
                    return;
                }
            }
        } else {
            match USVString::from_jsval(cx, value.handle(), ()) {
                Ok(ConversionResult::Success(USVString(string))) => {
                    ErrorInfo {
                        message: format!("uncaught exception: {}", string),
                        filename: String::new(),
                        lineno: 0,
                        column: 0,
                    }
                },
                _ => {
                    panic!("Uncaught exception: failed to stringify primitive");
                },
            }
        };

        error!("Error at {}:{}:{} {}",
               error_info.filename,
               error_info.lineno,
               error_info.column,
               error_info.message);

        if dispatch_event {
            let global = global_root_from_context(cx);
            if let GlobalRef::Window(window) = global.r() {
                window.report_an_error(error_info, value.handle());
            }
        }
    }
}
