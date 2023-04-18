File_Code/servo/283fc8dd25/devtools/devtools_after.rs --- 1/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
24 use js::jsapi::{ObjectClassName, RootedObject, RootedValue};                                                                                              24 use js::jsapi::{JSAutoCompartment, ObjectClassName, RootedObject, RootedValue};

File_Code/servo/283fc8dd25/devtools/devtools_after.rs --- 2/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
                                                                                                                                                             38         let globalhandle = global.reflector().get_jsobject();
                                                                                                                                                             39         let _ac = JSAutoCompartment::new(cx, globalhandle.get());

