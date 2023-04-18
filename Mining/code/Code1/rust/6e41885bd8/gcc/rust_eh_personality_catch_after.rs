    pub extern fn rust_eh_personality_catch(
        state: uw::_Unwind_State,
        ue_header: *mut uw::_Unwind_Exception,
        context: *mut uw::_Unwind_Context
    ) -> uw::_Unwind_Reason_Code
    {
        // Backtraces on ARM will call the personality routine with
        // state == _US_VIRTUAL_UNWIND_FRAME | _US_FORCE_UNWIND. In those cases
        // we want to continue unwinding the stack, otherwise all our backtraces
        // would end at __rust_try.
        if (state as c_int & uw::_US_ACTION_MASK as c_int)
                           == uw::_US_VIRTUAL_UNWIND_FRAME as c_int
               && (state as c_int & uw::_US_FORCE_UNWIND as c_int) == 0 { // search phase
            uw::_URC_HANDLER_FOUND // catch!
        }
        else { // cleanup phase
            unsafe {
                __gcc_personality_v0(state, ue_header, context)
            }
        }
    }
