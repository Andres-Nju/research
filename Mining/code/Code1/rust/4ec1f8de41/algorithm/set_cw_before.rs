    fn set_cw(cw: u16) {
        unsafe { asm!("fldcw $0" :: "m" (cw)) :: "volatile" }
    }
