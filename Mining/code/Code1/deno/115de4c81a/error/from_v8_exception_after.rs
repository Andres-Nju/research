  pub fn from_v8_exception(
    scope: &mut v8::HandleScope,
    exception: v8::Local<v8::Value>,
  ) -> Self {
    // Create a new HandleScope because we're creating a lot of new local
    // handles below.
    let scope = &mut v8::HandleScope::new(scope);

    let msg = v8::Exception::create_message(scope, exception);

    let (message, frames, stack) = if exception.is_native_error() {
      // The exception is a JS Error object.
      let exception: v8::Local<v8::Object> =
        exception.clone().try_into().unwrap();

      // Get the message by formatting error.name and error.message.
      let name = get_property(scope, exception, "name")
        .filter(|v| !v.is_undefined())
        .and_then(|m| m.to_string(scope))
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_else(|| "Error".to_string());
      let message_prop = get_property(scope, exception, "message")
        .filter(|v| !v.is_undefined())
        .and_then(|m| m.to_string(scope))
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_else(|| "".to_string());
      let message = if !name.is_empty() && !message_prop.is_empty() {
        format!("Uncaught {}: {}", name, message_prop)
      } else if !name.is_empty() {
        format!("Uncaught {}", name)
      } else if !message_prop.is_empty() {
        format!("Uncaught {}", message_prop)
      } else {
        "Uncaught".to_string()
      };

      // Access error.stack to ensure that prepareStackTrace() has been called.
      // This should populate error.__callSiteEvals.
      let stack: Option<v8::Local<v8::String>> =
        get_property(scope, exception, "stack")
          .unwrap()
          .try_into()
          .ok();
      let stack = stack.map(|s| s.to_rust_string_lossy(scope));

      // FIXME(bartlmieju): the rest of this function is CLI only

      // Read an array of structured frames from error.__callSiteEvals.
      let frames_v8 = get_property(scope, exception, "__callSiteEvals");
      let frames_v8: Option<v8::Local<v8::Array>> =
        frames_v8.and_then(|a| a.try_into().ok());

      // Convert them into Vec<JSStack> and Vec<String> respectively.
      let mut frames: Vec<JsStackFrame> = vec![];
      if let Some(frames_v8) = frames_v8 {
        for i in 0..frames_v8.length() {
          let call_site: v8::Local<v8::Object> =
            frames_v8.get_index(scope, i).unwrap().try_into().unwrap();
          let type_name: Option<v8::Local<v8::String>> =
            get_property(scope, call_site, "typeName")
              .unwrap()
              .try_into()
              .ok();
          let type_name = type_name.map(|s| s.to_rust_string_lossy(scope));
          let function_name: Option<v8::Local<v8::String>> =
            get_property(scope, call_site, "functionName")
              .unwrap()
              .try_into()
              .ok();
          let function_name =
            function_name.map(|s| s.to_rust_string_lossy(scope));
          let method_name: Option<v8::Local<v8::String>> =
            get_property(scope, call_site, "methodName")
              .unwrap()
              .try_into()
              .ok();
          let method_name = method_name.map(|s| s.to_rust_string_lossy(scope));
          let file_name: Option<v8::Local<v8::String>> =
            get_property(scope, call_site, "fileName")
              .unwrap()
              .try_into()
              .ok();
          let file_name = file_name.map(|s| s.to_rust_string_lossy(scope));
          let line_number: Option<v8::Local<v8::Integer>> =
            get_property(scope, call_site, "lineNumber")
              .unwrap()
              .try_into()
              .ok();
          let line_number = line_number.map(|n| n.value());
          let column_number: Option<v8::Local<v8::Integer>> =
            get_property(scope, call_site, "columnNumber")
              .unwrap()
              .try_into()
              .ok();
          let column_number = column_number.map(|n| n.value());
          let eval_origin: Option<v8::Local<v8::String>> =
            get_property(scope, call_site, "evalOrigin")
              .unwrap()
              .try_into()
              .ok();
          let eval_origin = eval_origin.map(|s| s.to_rust_string_lossy(scope));
          let is_top_level: Option<v8::Local<v8::Boolean>> =
            get_property(scope, call_site, "isToplevel")
              .unwrap()
              .try_into()
              .ok();
          let is_top_level = is_top_level.map(|b| b.is_true());
          let is_eval: v8::Local<v8::Boolean> =
            get_property(scope, call_site, "isEval")
              .unwrap()
              .try_into()
              .unwrap();
          let is_eval = is_eval.is_true();
          let is_native: v8::Local<v8::Boolean> =
            get_property(scope, call_site, "isNative")
              .unwrap()
              .try_into()
              .unwrap();
          let is_native = is_native.is_true();
          let is_constructor: v8::Local<v8::Boolean> =
            get_property(scope, call_site, "isConstructor")
              .unwrap()
              .try_into()
              .unwrap();
          let is_constructor = is_constructor.is_true();
          let is_async: v8::Local<v8::Boolean> =
            get_property(scope, call_site, "isAsync")
              .unwrap()
              .try_into()
              .unwrap();
          let is_async = is_async.is_true();
          let is_promise_all: v8::Local<v8::Boolean> =
            get_property(scope, call_site, "isPromiseAll")
              .unwrap()
              .try_into()
              .unwrap();
          let is_promise_all = is_promise_all.is_true();
          let promise_index: Option<v8::Local<v8::Integer>> =
            get_property(scope, call_site, "promiseIndex")
              .unwrap()
              .try_into()
              .ok();
          let promise_index = promise_index.map(|n| n.value());
          frames.push(JsStackFrame {
            type_name,
            function_name,
            method_name,
            file_name,
            line_number,
            column_number,
            eval_origin,
            is_top_level,
            is_eval,
            is_native,
            is_constructor,
            is_async,
            is_promise_all,
            promise_index,
          });
        }
      }
      (message, frames, stack)
    } else {
      // The exception is not a JS Error object.
      // Get the message given by V8::Exception::create_message(), and provide
      // empty frames.
      (msg.get(scope).to_rust_string_lossy(scope), vec![], None)
    };

    Self {
      message,
      script_resource_name: msg
        .get_script_resource_name(scope)
        .and_then(|v| v8::Local::<v8::String>::try_from(v).ok())
        .map(|v| v.to_rust_string_lossy(scope)),
      source_line: msg
        .get_source_line(scope)
        .map(|v| v.to_rust_string_lossy(scope)),
      line_number: msg.get_line_number(scope).and_then(|v| v.try_into().ok()),
      start_column: msg.get_start_column().try_into().ok(),
      end_column: msg.get_end_column().try_into().ok(),
      frames,
      stack,
    }
  }
