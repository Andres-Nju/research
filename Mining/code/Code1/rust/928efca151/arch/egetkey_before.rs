pub fn egetkey(request: &Align512<[u8; 512]>) -> Result<Align16<[u8; 16]>, u32> {
    unsafe {
        let mut out = MaybeUninit::uninitialized();
        let error;

        asm!(
            "enclu"
            : "={eax}"(error)
            : "{eax}"(ENCLU_EGETKEY),
              "{rbx}"(request),
              "{rcx}"(out.get_mut())
            : "flags"
        );

        match error {
            0 => Ok(out.into_inner()),
            err => Err(err),
        }
    }
}
