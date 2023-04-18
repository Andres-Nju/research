    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ty) = self.as_type() {
            write!(f, "{:?}", ty)
        } else if let Some(r) = self.as_region() {
            write!(f, "{:?}", r)
        } else {
            write!(f, "<unknown @ {:p}>", self.ptr.get() as *const ())
        }
    }
