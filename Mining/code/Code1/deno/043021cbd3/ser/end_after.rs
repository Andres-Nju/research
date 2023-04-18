  fn end(self) -> JsResult<'a> {
    self.end(S::end)
  }
}

pub struct ArraySerializer<'a, 'b, 'c> {
  pending: Vec<JsValue<'a>>,
  scope: ScopePtr<'a, 'b, 'c>,
}

