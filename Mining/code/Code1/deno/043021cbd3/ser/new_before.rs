  pub fn new(
    scope: ScopePtr<'a, 'b, 'c>,
    variant: &'static str,
    inner: S,
  ) -> Self {
    Self {
      scope,
      variant,
      inner,
    }
  }
