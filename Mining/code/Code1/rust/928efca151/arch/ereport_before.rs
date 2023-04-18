pub fn ereport(
    targetinfo: &Align512<[u8; 512]>,
    reportdata: &Align128<[u8; 64]>,
) -> Align512<[u8; 432]> {
    unsafe {
        let mut report = MaybeUninit::uninitialized();

        asm!(
            "enclu"
            : /* no output registers */
            : "{eax}"(ENCLU_EREPORT),
              "{rbx}"(targetinfo),
              "{rcx}"(reportdata),
              "{rdx}"(report.get_mut())
        );

        report.into_inner()
    }
}
