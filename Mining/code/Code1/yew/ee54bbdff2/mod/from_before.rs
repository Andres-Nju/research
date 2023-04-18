    fn from(s: &'static str) -> Self {
        AttrValue::Static(s)
    }
}

impl From<String> for AttrValue {
    fn from(s: String) -> Self {
        AttrValue::Owned(s)
    }
}

impl From<Rc<str>> for AttrValue {
    fn from(s: Rc<str>) -> Self {
        AttrValue::Rc(s)
    }
}

impl Clone for AttrValue {
    fn clone(&self) -> Self {
        match self {
            AttrValue::Static(s) => AttrValue::Static(s),
            AttrValue::Owned(s) => AttrValue::Owned(s.clone()),
            AttrValue::Rc(s) => AttrValue::Rc(Rc::clone(s)),
        }
    }
}

impl AsRef<str> for AttrValue {
    fn as_ref(&self) -> &str {
        &*self
    }
}

impl fmt::Display for AttrValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
