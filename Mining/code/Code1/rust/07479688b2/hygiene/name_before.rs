    pub fn name(self) -> Symbol {
        Symbol::intern(match self {
            CompilerDesugaringKind::Async => "async",
            CompilerDesugaringKind::DotFill => "...",
            CompilerDesugaringKind::QuestionMark => "?",
            CompilerDesugaringKind::Catch => "do catch",
            CompilerDesugaringKind::ExistentialReturnType => "existental type",
        })
    }
