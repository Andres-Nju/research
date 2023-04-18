    fn to_ast(self) -> Self::Ast;
}

impl<'a> ToAst for &'a OwnedAst<ast::FnDef<'static>> {
    type Ast = ast::FnDef<'a>;
    fn to_ast(self) -> ast::FnDef<'a> {
        ast::FnDef::cast(self.syntax.borrowed())
            .unwrap()
    }
}


/// A pionter to a syntax node inside a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct LocalSyntaxPtr {
    range: TextRange,
    kind: SyntaxKind,
}

impl LocalSyntaxPtr {
    pub(crate) fn new(node: SyntaxNodeRef) -> LocalSyntaxPtr {
        LocalSyntaxPtr {
            range: node.range(),
            kind: node.kind(),
        }
    }

    pub(crate) fn resolve(self, file: &File) -> SyntaxNode {
        let mut curr = file.syntax();
        loop {
            if curr.range() == self.range && curr.kind() == self.kind {
                return curr.owned();
            }
            curr = curr.children()
                .find(|it| self.range.is_subrange(&it.range()))
                .unwrap_or_else(|| panic!("can't resovle local ptr to SyntaxNode: {:?}", self))
