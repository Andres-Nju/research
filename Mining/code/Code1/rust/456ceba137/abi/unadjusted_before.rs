    pub fn unadjusted<'a, 'tcx>(ccx: &CrateContext<'a, 'tcx>,
                                abi: Abi,
                                sig: &ty::FnSig<'tcx>,
                                extra_args: &[Ty<'tcx>]) -> FnType {
        use self::Abi::*;
        let cconv = match ccx.sess().target.target.adjust_abi(abi) {
            RustIntrinsic | PlatformIntrinsic |
            Rust | RustCall => llvm::CCallConv,

            // It's the ABI's job to select this, not us.
            System => bug!("system abi should be selected elsewhere"),

            Stdcall => llvm::X86StdcallCallConv,
            Fastcall => llvm::X86FastcallCallConv,
            Vectorcall => llvm::X86_VectorCall,
            C => llvm::CCallConv,
            Win64 => llvm::X86_64_Win64,
            SysV64 => llvm::X86_64_SysV,

            // These API constants ought to be more specific...
            Cdecl => llvm::CCallConv,
            Aapcs => llvm::CCallConv,
        };

        let mut inputs = &sig.inputs[..];
        let extra_args = if abi == RustCall {
            assert!(!sig.variadic && extra_args.is_empty());

            match inputs[inputs.len() - 1].sty {
                ty::TyTuple(ref tupled_arguments) => {
                    inputs = &inputs[..inputs.len() - 1];
                    &tupled_arguments[..]
                }
                _ => {
                    bug!("argument to function with \"rust-call\" ABI \
                          is not a tuple");
                }
            }
        } else {
            assert!(sig.variadic || extra_args.is_empty());
            extra_args
        };

        let target = &ccx.sess().target.target;
        let win_x64_gnu = target.target_os == "windows"
                       && target.arch == "x86_64"
                       && target.target_env == "gnu";
        let linux_s390x = target.target_os == "linux"
                       && target.arch == "s390x"
                       && target.target_env == "gnu";
        let rust_abi = match abi {
            RustIntrinsic | PlatformIntrinsic | Rust | RustCall => true,
            _ => false
        };

        let arg_of = |ty: Ty<'tcx>, is_return: bool| {
            if ty.is_bool() {
                let llty = Type::i1(ccx);
                let mut arg = ArgType::new(llty, llty);
                arg.attrs.set(llvm::Attribute::ZExt);
                arg
            } else {
                let mut arg = ArgType::new(type_of::type_of(ccx, ty),
                                           type_of::sizing_type_of(ccx, ty));
                if ty.is_integral() {
                    arg.signedness = Some(ty.is_signed());
                }
                // Rust enum types that map onto C enums also need to follow
                // the target ABI zero-/sign-extension rules.
                if let Layout::CEnum { signed, .. } = *ccx.layout_of(ty) {
                    arg.signedness = Some(signed);
                }
                if llsize_of_alloc(ccx, arg.ty) == 0 {
                    // For some forsaken reason, x86_64-pc-windows-gnu
                    // doesn't ignore zero-sized struct arguments.
                    // The same is true for s390x-unknown-linux-gnu.
                    if is_return || rust_abi ||
                       (!win_x64_gnu && !linux_s390x) {
                        arg.ignore();
                    }
                }
