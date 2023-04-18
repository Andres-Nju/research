    pub fn load(&self, ptr: &'ll Value, align: Align) -> &'ll Value {
        self.count_insn("load");
        unsafe {
            let load = llvm::LLVMBuildLoad(self.llbuilder, ptr, noname());
            llvm::LLVMSetAlignment(load, align.abi() as c_uint);
            load
        }
    }

    pub fn volatile_load(&self, ptr: &'ll Value) -> &'ll Value {
        self.count_insn("load.volatile");
        unsafe {
            let insn = llvm::LLVMBuildLoad(self.llbuilder, ptr, noname());
            llvm::LLVMSetVolatile(insn, llvm::True);
            insn
        }
    }

    pub fn atomic_load(&self, ptr: &'ll Value, order: AtomicOrdering, align: Align) -> &'ll Value {
        self.count_insn("load.atomic");
        unsafe {
            let load = llvm::LLVMRustBuildAtomicLoad(self.llbuilder, ptr, noname(), order);
            // FIXME(eddyb) Isn't it UB to use `pref` instead of `abi` here?
            // However, 64-bit atomic loads on `i686-apple-darwin` appear to
            // require `___atomic_load` with ABI-alignment, so it's staying.
            llvm::LLVMSetAlignment(load, align.pref() as c_uint);
            load
        }
    }


    pub fn range_metadata(&self, load: &'ll Value, range: Range<u128>) {
        if self.sess().target.target.arch == "amdgpu" {
            // amdgpu/LLVM does something weird and thinks a i64 value is
            // split into a v2i32, halving the bitwidth LLVM expects,
            // tripping an assertion. So, for now, just disable this
            // optimization.
            return;
        }

        unsafe {
            let llty = val_ty(load);
            let v = [
                C_uint_big(llty, range.start),
                C_uint_big(llty, range.end)
            ];

            llvm::LLVMSetMetadata(load, llvm::MD_range as c_uint,
                                  llvm::LLVMMDNodeInContext(self.cx.llcx,
                                                            v.as_ptr(),
                                                            v.len() as c_uint));
        }
    }

    pub fn nonnull_metadata(&self, load: &'ll Value) {
        unsafe {
            llvm::LLVMSetMetadata(load, llvm::MD_nonnull as c_uint,
                                  llvm::LLVMMDNodeInContext(self.cx.llcx, ptr::null(), 0));
        }
    }

    pub fn store(&self, val: &'ll Value, ptr: &'ll Value, align: Align) -> &'ll Value {
        self.store_with_flags(val, ptr, align, MemFlags::empty())
    }

    pub fn store_with_flags(
        &self,
        val: &'ll Value,
        ptr: &'ll Value,
        align: Align,
        flags: MemFlags,
    ) -> &'ll Value {
        debug!("Store {:?} -> {:?} ({:?})", val, ptr, flags);
        self.count_insn("store");
        let ptr = self.check_store(val, ptr);
        unsafe {
            let store = llvm::LLVMBuildStore(self.llbuilder, val, ptr);
            let align = if flags.contains(MemFlags::UNALIGNED) {
                1
            } else {
                align.abi() as c_uint
            };
            llvm::LLVMSetAlignment(store, align);
            if flags.contains(MemFlags::VOLATILE) {
                llvm::LLVMSetVolatile(store, llvm::True);
            }
            if flags.contains(MemFlags::NONTEMPORAL) {
                // According to LLVM [1] building a nontemporal store must
                // *always* point to a metadata value of the integer 1.
                //
                // [1]: http://llvm.org/docs/LangRef.html#store-instruction
                let one = C_i32(self.cx, 1);
                let node = llvm::LLVMMDNodeInContext(self.cx.llcx, &one, 1);
                llvm::LLVMSetMetadata(store, llvm::MD_nontemporal as c_uint, node);
            }
            store
        }
    }

    pub fn atomic_store(&self, val: &'ll Value, ptr: &'ll Value,
                        order: AtomicOrdering, align: Align) {
        debug!("Store {:?} -> {:?}", val, ptr);
        self.count_insn("store.atomic");
        let ptr = self.check_store(val, ptr);
        unsafe {
            let store = llvm::LLVMRustBuildAtomicStore(self.llbuilder, val, ptr, order);
            // FIXME(eddyb) Isn't it UB to use `pref` instead of `abi` here?
            // Also see `atomic_load` for more context.
            llvm::LLVMSetAlignment(store, align.pref() as c_uint);
        }
    }

    pub fn gep(&self, ptr: &'ll Value, indices: &[&'ll Value]) -> &'ll Value {
        self.count_insn("gep");
        unsafe {
            llvm::LLVMBuildGEP(self.llbuilder, ptr, indices.as_ptr(),
                               indices.len() as c_uint, noname())
        }
    }

    pub fn inbounds_gep(&self, ptr: &'ll Value, indices: &[&'ll Value]) -> &'ll Value {
        self.count_insn("inboundsgep");
        unsafe {
            llvm::LLVMBuildInBoundsGEP(
                self.llbuilder, ptr, indices.as_ptr(), indices.len() as c_uint, noname())
        }
    }

    pub fn struct_gep(&self, ptr: &'ll Value, idx: u64) -> &'ll Value {
        self.count_insn("structgep");
        assert_eq!(idx as c_uint as u64, idx);
        unsafe {
            llvm::LLVMBuildStructGEP(self.llbuilder, ptr, idx as c_uint, noname())
        }
    }

    /* Casts */
    pub fn trunc(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("trunc");
        unsafe {
            llvm::LLVMBuildTrunc(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn zext(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("zext");
        unsafe {
            llvm::LLVMBuildZExt(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn sext(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("sext");
        unsafe {
            llvm::LLVMBuildSExt(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn fptoui(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("fptoui");
        unsafe {
            llvm::LLVMBuildFPToUI(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn fptosi(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("fptosi");
        unsafe {
            llvm::LLVMBuildFPToSI(self.llbuilder, val, dest_ty,noname())
        }
    }

    pub fn uitofp(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("uitofp");
        unsafe {
            llvm::LLVMBuildUIToFP(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn sitofp(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("sitofp");
        unsafe {
            llvm::LLVMBuildSIToFP(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn fptrunc(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("fptrunc");
        unsafe {
            llvm::LLVMBuildFPTrunc(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn fpext(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("fpext");
        unsafe {
            llvm::LLVMBuildFPExt(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn ptrtoint(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("ptrtoint");
        unsafe {
            llvm::LLVMBuildPtrToInt(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn inttoptr(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("inttoptr");
        unsafe {
            llvm::LLVMBuildIntToPtr(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn bitcast(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("bitcast");
        unsafe {
            llvm::LLVMBuildBitCast(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn pointercast(&self, val: &'ll Value, dest_ty: &'ll Type) -> &'ll Value {
        self.count_insn("pointercast");
        unsafe {
            llvm::LLVMBuildPointerCast(self.llbuilder, val, dest_ty, noname())
        }
    }

    pub fn intcast(&self, val: &'ll Value, dest_ty: &'ll Type, is_signed: bool) -> &'ll Value {
        self.count_insn("intcast");
        unsafe {
            llvm::LLVMRustBuildIntCast(self.llbuilder, val, dest_ty, is_signed)
        }
    }

    /* Comparisons */
    pub fn icmp(&self, op: IntPredicate, lhs: &'ll Value, rhs: &'ll Value) -> &'ll Value {
        self.count_insn("icmp");
        unsafe {
            llvm::LLVMBuildICmp(self.llbuilder, op as c_uint, lhs, rhs, noname())
        }
    }

    pub fn fcmp(&self, op: RealPredicate, lhs: &'ll Value, rhs: &'ll Value) -> &'ll Value {
        self.count_insn("fcmp");
        unsafe {
            llvm::LLVMBuildFCmp(self.llbuilder, op as c_uint, lhs, rhs, noname())
        }
    }

    /* Miscellaneous instructions */
    pub fn empty_phi(&self, ty: &'ll Type) -> &'ll Value {
        self.count_insn("emptyphi");
        unsafe {
            llvm::LLVMBuildPhi(self.llbuilder, ty, noname())
        }
    }

    pub fn phi(&self, ty: &'ll Type, vals: &[&'ll Value], bbs: &[&'ll BasicBlock]) -> &'ll Value {
        assert_eq!(vals.len(), bbs.len());
        let phi = self.empty_phi(ty);
        self.count_insn("addincoming");
        unsafe {
            llvm::LLVMAddIncoming(phi, vals.as_ptr(),
                                  bbs.as_ptr(),
                                  vals.len() as c_uint);
            phi
        }
    }

    pub fn inline_asm_call(&self, asm: *const c_char, cons: *const c_char,
                           inputs: &[&'ll Value], output: &'ll Type,
                           volatile: bool, alignstack: bool,
                           dia: AsmDialect) -> Option<&'ll Value> {
        self.count_insn("inlineasm");

        let volatile = if volatile { llvm::True }
                       else        { llvm::False };
        let alignstack = if alignstack { llvm::True }
                         else          { llvm::False };

        let argtys = inputs.iter().map(|v| {
            debug!("Asm Input Type: {:?}", *v);
            val_ty(*v)
        }).collect::<Vec<_>>();

        debug!("Asm Output Type: {:?}", output);
        let fty = Type::func(&argtys[..], output);
        unsafe {
            // Ask LLVM to verify that the constraints are well-formed.
            let constraints_ok = llvm::LLVMRustInlineAsmVerify(fty, cons);
            debug!("Constraint verification result: {:?}", constraints_ok);
            if constraints_ok {
                let v = llvm::LLVMRustInlineAsm(
                    fty, asm, cons, volatile, alignstack, dia);
                Some(self.call(v, inputs, None))
            } else {
                // LLVM has detected an issue with our constaints, bail out
                None
            }
        }
    }

    pub fn call(&self, llfn: &'ll Value, args: &[&'ll Value],
                bundle: Option<&OperandBundleDef<'ll>>) -> &'ll Value {
        self.count_insn("call");

        debug!("Call {:?} with args ({:?})",
               llfn,
               args);

        let args = self.check_call("call", llfn, args);
        let bundle = bundle.map(|b| &*b.raw);

        unsafe {
            llvm::LLVMRustBuildCall(self.llbuilder, llfn, args.as_ptr(),
                                    args.len() as c_uint, bundle, noname())
        }
    }

    pub fn minnum(&self, lhs: &'ll Value, rhs: &'ll Value) -> &'ll Value {
        self.count_insn("minnum");
        unsafe {
            let instr = llvm::LLVMRustBuildMinNum(self.llbuilder, lhs, rhs);
            instr.expect("LLVMRustBuildMinNum is not available in LLVM version < 6.0")
        }
    }
    pub fn maxnum(&self, lhs: &'ll Value, rhs: &'ll Value) -> &'ll Value {
        self.count_insn("maxnum");
        unsafe {
            let instr = llvm::LLVMRustBuildMaxNum(self.llbuilder, lhs, rhs);
            instr.expect("LLVMRustBuildMaxNum is not available in LLVM version < 6.0")
        }
    }

    pub fn select(
        &self, cond: &'ll Value,
        then_val: &'ll Value,
        else_val: &'ll Value,
    ) -> &'ll Value {
        self.count_insn("select");
        unsafe {
            llvm::LLVMBuildSelect(self.llbuilder, cond, then_val, else_val, noname())
        }
    }

    #[allow(dead_code)]
    pub fn va_arg(&self, list: &'ll Value, ty: &'ll Type) -> &'ll Value {
        self.count_insn("vaarg");
        unsafe {
            llvm::LLVMBuildVAArg(self.llbuilder, list, ty, noname())
        }
    }

    pub fn extract_element(&self, vec: &'ll Value, idx: &'ll Value) -> &'ll Value {
        self.count_insn("extractelement");
        unsafe {
            llvm::LLVMBuildExtractElement(self.llbuilder, vec, idx, noname())
        }
    }

    pub fn insert_element(
        &self, vec: &'ll Value,
        elt: &'ll Value,
        idx: &'ll Value,
    ) -> &'ll Value {
        self.count_insn("insertelement");
        unsafe {
            llvm::LLVMBuildInsertElement(self.llbuilder, vec, elt, idx, noname())
        }
    }

    pub fn shuffle_vector(&self, v1: &'ll Value, v2: &'ll Value, mask: &'ll Value) -> &'ll Value {
        self.count_insn("shufflevector");
        unsafe {
            llvm::LLVMBuildShuffleVector(self.llbuilder, v1, v2, mask, noname())
        }
    }

    pub fn vector_splat(&self, num_elts: usize, elt: &'ll Value) -> &'ll Value {
        unsafe {
            let elt_ty = val_ty(elt);
            let undef = llvm::LLVMGetUndef(Type::vector(elt_ty, num_elts as u64));
            let vec = self.insert_element(undef, elt, C_i32(self.cx, 0));
            let vec_i32_ty = Type::vector(Type::i32(self.cx), num_elts as u64);
            self.shuffle_vector(vec, undef, C_null(vec_i32_ty))
        }
    }

    pub fn vector_reduce_fadd_fast(&self, acc: &'ll Value, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.fadd_fast");
        unsafe {
            // FIXME: add a non-fast math version once
            // https://bugs.llvm.org/show_bug.cgi?id=36732
            // is fixed.
            let instr = llvm::LLVMRustBuildVectorReduceFAdd(self.llbuilder, acc, src)
                .expect("LLVMRustBuildVectorReduceFAdd is not available in LLVM version < 5.0");
            llvm::LLVMRustSetHasUnsafeAlgebra(instr);
            instr
        }
    }
    pub fn vector_reduce_fmul_fast(&self, acc: &'ll Value, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.fmul_fast");
        unsafe {
            // FIXME: add a non-fast math version once
            // https://bugs.llvm.org/show_bug.cgi?id=36732
            // is fixed.
            let instr = llvm::LLVMRustBuildVectorReduceFMul(self.llbuilder, acc, src)
                .expect("LLVMRustBuildVectorReduceFMul is not available in LLVM version < 5.0");
            llvm::LLVMRustSetHasUnsafeAlgebra(instr);
            instr
        }
    }
    pub fn vector_reduce_add(&self, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.add");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceAdd(self.llbuilder, src);
            instr.expect("LLVMRustBuildVectorReduceAdd is not available in LLVM version < 5.0")
        }
    }
    pub fn vector_reduce_mul(&self, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.mul");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceMul(self.llbuilder, src);
            instr.expect("LLVMRustBuildVectorReduceMul is not available in LLVM version < 5.0")
        }
    }
    pub fn vector_reduce_and(&self, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.and");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceAnd(self.llbuilder, src);
            instr.expect("LLVMRustBuildVectorReduceAnd is not available in LLVM version < 5.0")
        }
    }
    pub fn vector_reduce_or(&self, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.or");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceOr(self.llbuilder, src);
            instr.expect("LLVMRustBuildVectorReduceOr is not available in LLVM version < 5.0")
        }
    }
    pub fn vector_reduce_xor(&self, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.xor");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceXor(self.llbuilder, src);
            instr.expect("LLVMRustBuildVectorReduceXor is not available in LLVM version < 5.0")
        }
    }
    pub fn vector_reduce_fmin(&self, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.fmin");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceFMin(self.llbuilder, src, /*NoNaNs:*/ false);
            instr.expect("LLVMRustBuildVectorReduceFMin is not available in LLVM version < 5.0")
        }
    }
    pub fn vector_reduce_fmax(&self, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.fmax");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceFMax(self.llbuilder, src, /*NoNaNs:*/ false);
            instr.expect("LLVMRustBuildVectorReduceFMax is not available in LLVM version < 5.0")
        }
    }
    pub fn vector_reduce_fmin_fast(&self, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.fmin_fast");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceFMin(self.llbuilder, src, /*NoNaNs:*/ true)
                .expect("LLVMRustBuildVectorReduceFMin is not available in LLVM version < 5.0");
            llvm::LLVMRustSetHasUnsafeAlgebra(instr);
            instr
        }
    }
    pub fn vector_reduce_fmax_fast(&self, src: &'ll Value) -> &'ll Value {
        self.count_insn("vector.reduce.fmax_fast");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceFMax(self.llbuilder, src, /*NoNaNs:*/ true)
                .expect("LLVMRustBuildVectorReduceFMax is not available in LLVM version < 5.0");
            llvm::LLVMRustSetHasUnsafeAlgebra(instr);
            instr
        }
    }
    pub fn vector_reduce_min(&self, src: &'ll Value, is_signed: bool) -> &'ll Value {
        self.count_insn("vector.reduce.min");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceMin(self.llbuilder, src, is_signed);
            instr.expect("LLVMRustBuildVectorReduceMin is not available in LLVM version < 5.0")
        }
    }
    pub fn vector_reduce_max(&self, src: &'ll Value, is_signed: bool) -> &'ll Value {
        self.count_insn("vector.reduce.max");
        unsafe {
            let instr = llvm::LLVMRustBuildVectorReduceMax(self.llbuilder, src, is_signed);
            instr.expect("LLVMRustBuildVectorReduceMax is not available in LLVM version < 5.0")
        }
    }

    pub fn extract_value(&self, agg_val: &'ll Value, idx: u64) -> &'ll Value {
        self.count_insn("extractvalue");
        assert_eq!(idx as c_uint as u64, idx);
        unsafe {
            llvm::LLVMBuildExtractValue(self.llbuilder, agg_val, idx as c_uint, noname())
        }
    }

    pub fn insert_value(&self, agg_val: &'ll Value, elt: &'ll Value,
                       idx: u64) -> &'ll Value {
        self.count_insn("insertvalue");
        assert_eq!(idx as c_uint as u64, idx);
        unsafe {
            llvm::LLVMBuildInsertValue(self.llbuilder, agg_val, elt, idx as c_uint,
                                       noname())
        }
    }
