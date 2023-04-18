pub fn handle_evaluate_js(global: &GlobalRef, eval: String, reply: IpcSender<EvaluateJSReply>) {
    // global.get_cx() returns a valid `JSContext` pointer, so this is safe.
    let result = unsafe {
        let cx = global.get_cx();
        let globalhandle = global.reflector().get_jsobject();
        let _ac = JSAutoCompartment::new(cx, globalhandle.get());
        let mut rval = RootedValue::new(cx, UndefinedValue());
        global.evaluate_js_on_global_with_result(&eval, rval.handle_mut());

        if rval.ptr.is_undefined() {
            EvaluateJSReply::VoidValue
        } else if rval.ptr.is_boolean() {
            EvaluateJSReply::BooleanValue(rval.ptr.to_boolean())
        } else if rval.ptr.is_double() || rval.ptr.is_int32() {
            EvaluateJSReply::NumberValue(FromJSValConvertible::from_jsval(cx, rval.handle(), ())
                                             .unwrap())
        } else if rval.ptr.is_string() {
            EvaluateJSReply::StringValue(String::from(jsstring_to_str(cx, rval.ptr.to_string())))
        } else if rval.ptr.is_null() {
            EvaluateJSReply::NullValue
        } else {
            assert!(rval.ptr.is_object());

            let obj = RootedObject::new(cx, rval.ptr.to_object());
            let class_name = CStr::from_ptr(ObjectClassName(cx, obj.handle()));
            let class_name = str::from_utf8(class_name.to_bytes()).unwrap();

            EvaluateJSReply::ActorValue {
                class: class_name.to_owned(),
                uuid: Uuid::new_v4().to_string(),
            }
        }
    };
    reply.send(result).unwrap();
}
