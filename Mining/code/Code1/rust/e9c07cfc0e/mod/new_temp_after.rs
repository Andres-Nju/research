    pub fn new_temp(ty: Ty<'tcx>) -> Self {
        LocalDecl {
            mutability: Mutability::Mut,
            ty: ty,
            name: None,
            source_info: None,
        }
    }

    /// Builds a `LocalDecl` for the return pointer.
    ///
    /// This must be inserted into the `local_decls` list as the first local.
    #[inline]
    pub fn new_return_pointer(return_ty: Ty) -> LocalDecl {
        LocalDecl {
            mutability: Mutability::Mut,
            ty: return_ty,
            source_info: None,
            name: None,     // FIXME maybe we do want some name here?
        }
    }
}

/// A closure capture, with its name and mode.
#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct UpvarDecl {
    pub debug_name: Name,

    /// If true, the capture is behind a reference.
    pub by_ref: bool
}

///////////////////////////////////////////////////////////////////////////
// BasicBlock

newtype_index!(BasicBlock, "bb");

///////////////////////////////////////////////////////////////////////////
// BasicBlockData and Terminator

#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct BasicBlockData<'tcx> {
    /// List of statements in this block.
    pub statements: Vec<Statement<'tcx>>,

    /// Terminator for this block.
    ///
    /// NB. This should generally ONLY be `None` during construction.
    /// Therefore, you should generally access it via the
    /// `terminator()` or `terminator_mut()` methods. The only
    /// exception is that certain passes, such as `simplify_cfg`, swap
    /// out the terminator temporarily with `None` while they continue
    /// to recurse over the set of basic blocks.
    pub terminator: Option<Terminator<'tcx>>,

    /// If true, this block lies on an unwind path. This is used
    /// during trans where distinct kinds of basic blocks may be
    /// generated (particularly for MSVC cleanup). Unwind blocks must
    /// only branch to other unwind blocks.
    pub is_cleanup: bool,
}

#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct Terminator<'tcx> {
    pub source_info: SourceInfo,
    pub kind: TerminatorKind<'tcx>
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum TerminatorKind<'tcx> {
    /// block should have one successor in the graph; we jump there
    Goto {
        target: BasicBlock,
    },

    /// operand evaluates to an integer; jump depending on its value
    /// to one of the targets, and otherwise fallback to `otherwise`
    SwitchInt {
        /// discriminant value being tested
        discr: Operand<'tcx>,

        /// type of value being tested
        switch_ty: Ty<'tcx>,

        /// Possible values. The locations to branch to in each case
        /// are found in the corresponding indices from the `targets` vector.
        values: Cow<'tcx, [ConstInt]>,

        /// Possible branch sites. The last element of this vector is used
        /// for the otherwise branch, so targets.len() == values.len() + 1
        /// should hold.
        // This invariant is quite non-obvious and also could be improved.
        // One way to make this invariant is to have something like this instead:
        //
        // branches: Vec<(ConstInt, BasicBlock)>,
        // otherwise: Option<BasicBlock> // exhaustive if None
        //
        // However we’ve decided to keep this as-is until we figure a case
        // where some other approach seems to be strictly better than other.
        targets: Vec<BasicBlock>,
    },

    /// Indicates that the landing pad is finished and unwinding should
    /// continue. Emitted by build::scope::diverge_cleanup.
    Resume,

    /// Indicates a normal return. The return pointer lvalue should
    /// have been filled in by now. This should occur at most once.
    Return,

    /// Indicates a terminator that can never be reached.
    Unreachable,

    /// Drop the Lvalue
    Drop {
        location: Lvalue<'tcx>,
        target: BasicBlock,
        unwind: Option<BasicBlock>
    },

    /// Drop the Lvalue and assign the new value over it
    DropAndReplace {
        location: Lvalue<'tcx>,
        value: Operand<'tcx>,
        target: BasicBlock,
        unwind: Option<BasicBlock>,
    },

    /// Block ends with a call of a converging function
    Call {
        /// The function that’s being called
        func: Operand<'tcx>,
        /// Arguments the function is called with
        args: Vec<Operand<'tcx>>,
        /// Destination for the return value. If some, the call is converging.
        destination: Option<(Lvalue<'tcx>, BasicBlock)>,
        /// Cleanups to be done if the call unwinds.
        cleanup: Option<BasicBlock>
    },

    /// Jump to the target if the condition has the expected value,
    /// otherwise panic with a message and a cleanup target.
    Assert {
        cond: Operand<'tcx>,
        expected: bool,
        msg: AssertMessage<'tcx>,
        target: BasicBlock,
        cleanup: Option<BasicBlock>
    }
