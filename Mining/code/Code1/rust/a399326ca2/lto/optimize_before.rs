    unsafe fn optimize(&mut self, cgcx: &CodegenContext, timeline: &mut Timeline)
        -> Result<ModuleTranslation, FatalError>
    {
        let diag_handler = cgcx.create_diag_handler();
        let tm = (cgcx.tm_factory)().map_err(|e| {
            write::llvm_err(&diag_handler, e)
        })?;

        // Right now the implementation we've got only works over serialized
        // modules, so we create a fresh new LLVM context and parse the module
        // into that context. One day, however, we may do this for upstream
        // crates but for locally translated modules we may be able to reuse
        // that LLVM Context and Module.
        let llcx = llvm::LLVMContextCreate();
        let llmod = llvm::LLVMRustParseBitcodeForThinLTO(
            llcx,
            self.data().as_ptr(),
            self.data().len(),
            self.shared.module_names[self.idx].as_ptr(),
        );
        assert!(!llmod.is_null());
        let mtrans = ModuleTranslation {
            source: ModuleSource::Translated(ModuleLlvm {
                llmod,
                llcx,
                tm,
            }),
            llmod_id: self.name().to_string(),
            name: self.name().to_string(),
            kind: ModuleKind::Regular,
        };
        cgcx.save_temp_bitcode(&mtrans, "thin-lto-input");

        // Like with "fat" LTO, get some better optimizations if landing pads
        // are disabled by removing all landing pads.
        if cgcx.no_landing_pads {
            llvm::LLVMRustMarkAllFunctionsNounwind(llmod);
            cgcx.save_temp_bitcode(&mtrans, "thin-lto-after-nounwind");
            timeline.record("nounwind");
        }

        // Up next comes the per-module local analyses that we do for Thin LTO.
        // Each of these functions is basically copied from the LLVM
        // implementation and then tailored to suit this implementation. Ideally
        // each of these would be supported by upstream LLVM but that's perhaps
        // a patch for another day!
        //
        // You can find some more comments about these functions in the LLVM
        // bindings we've got (currently `PassWrapper.cpp`)
        if !llvm::LLVMRustPrepareThinLTORename(self.shared.data.0, llmod) {
            let msg = format!("failed to prepare thin LTO module");
            return Err(write::llvm_err(&diag_handler, msg))
        }
        cgcx.save_temp_bitcode(&mtrans, "thin-lto-after-rename");
        timeline.record("rename");
        if !llvm::LLVMRustPrepareThinLTOResolveWeak(self.shared.data.0, llmod) {
            let msg = format!("failed to prepare thin LTO module");
            return Err(write::llvm_err(&diag_handler, msg))
        }
        cgcx.save_temp_bitcode(&mtrans, "thin-lto-after-resolve");
        timeline.record("resolve");
        if !llvm::LLVMRustPrepareThinLTOInternalize(self.shared.data.0, llmod) {
            let msg = format!("failed to prepare thin LTO module");
            return Err(write::llvm_err(&diag_handler, msg))
        }
        cgcx.save_temp_bitcode(&mtrans, "thin-lto-after-internalize");
        timeline.record("internalize");
        if !llvm::LLVMRustPrepareThinLTOImport(self.shared.data.0, llmod) {
            let msg = format!("failed to prepare thin LTO module");
            return Err(write::llvm_err(&diag_handler, msg))
        }
        cgcx.save_temp_bitcode(&mtrans, "thin-lto-after-import");
        timeline.record("import");

        // Alright now that we've done everything related to the ThinLTO
        // analysis it's time to run some optimizations! Here we use the same
        // `run_pass_manager` as the "fat" LTO above except that we tell it to
        // populate a thin-specific pass manager, which presumably LLVM treats a
        // little differently.
        info!("running thin lto passes over {}", mtrans.name);
        let config = cgcx.config(mtrans.kind);
        run_pass_manager(cgcx, tm, llmod, config, true);
        cgcx.save_temp_bitcode(&mtrans, "thin-lto-after-pm");
        timeline.record("thin-done");
        Ok(mtrans)
    }
