    pub fn eq_token(&self) -> Option<SyntaxToken> {
        self.syntax()
            .descendants_with_tokens()
            .find(|t| t.kind() == EQ)
            .and_then(|it| it.into_token())
    }
