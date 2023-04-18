    fn base_url(&self) -> ServoUrl {
        match *self {
            CSSStyleOwner::Element(ref el) => window_from_node(&**el).Document().base_url(),
            CSSStyleOwner::CSSRule(ref rule, _) => {
                rule.parent_stylesheet().style_stylesheet().base_url.clone()
            }
        }
    }
