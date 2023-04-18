    pub fn set_precision<T>() -> FPUControlWord {
        let cw = 0u16;

        // Compute the value for the Precision Control field that is appropriate for `T`.
        let cw_precision = match size_of::<T>() {
            4 => 0x0000, // 32 bits
            8 => 0x0200, // 64 bits
            _ => 0x0300, // default, 80 bits
        };

        // Get the original value of the control word to restore it later, when the
        // `FPUControlWord` structure is dropped
        unsafe { asm!("fnstcw $0" : "=*m" (&cw)) ::: "volatile" }

        // Set the control word to the desired precision. This is achieved by masking away the old
        // precision (bits 8 and 9, 0x300) and replacing it with the precision flag computed above.
        set_cw((cw & 0xFCFF) | cw_precision);

        FPUControlWord(cw)
    }
