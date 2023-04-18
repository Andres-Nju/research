    pub fn check_cfg_attributes(&self, warnings: &mut Vec<String>) {
        fn check_cfg_expr(expr: &CfgExpr, warnings: &mut Vec<String>) {
            match *expr {
                CfgExpr::Not(ref e) => check_cfg_expr(e, warnings),
                CfgExpr::All(ref e) | CfgExpr::Any(ref e) => {
                    for e in e {
                        check_cfg_expr(e, warnings);
                    }
                }
                CfgExpr::Value(ref e) => match e {
                    Cfg::Name(name) => match name.as_str() {
                        "test" | "debug_assertions" | "proc_macro" =>
                            warnings.push(format!(
                                "Found `{}` in `target.'cfg(...)'.dependencies`. \
                                 This value is not supported for selecting dependencies \
                                 and will not work as expected. \
                                 To learn more visit \
                                 https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies",
                                 name
                            )),
                        _ => (),
                    },
                    Cfg::KeyPair(name, _) => match name.as_str() {
                        "feature" =>
                            warnings.push(String::from(
                                "Found `feature = ...` in `target.'cfg(...)'.dependencies`. \
                                 This key is not supported for selecting dependencies \
                                 and will not work as expected. \
                                 Use the [features] section instead: \
                                 https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section"
                            )),
                        _ => (),
                    },
                }
            }
        }

        if let Platform::Cfg(cfg) = self {
            check_cfg_expr(cfg, warnings);
        }
    }
