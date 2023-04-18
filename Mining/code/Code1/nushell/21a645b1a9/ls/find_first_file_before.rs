    fn find_first_file(filename: &Path, span: Span) -> Result<WIN32_FIND_DATAW, ShellError> {
        unsafe {
            let mut find_data = MaybeUninit::<WIN32_FIND_DATAW>::uninit();
            // The windows crate really needs a nicer way to do string conversions
            let filename_wide: Vec<u16> = filename
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            if FindFirstFileW(
                windows::core::PCWSTR(filename_wide.as_ptr()),
                find_data.as_mut_ptr(),
            )
            .is_err()
            {
                return Err(ShellError::ReadingFile(
                    "Could not read file metadata".to_string(),
                    span,
                ));
            }

            let find_data = find_data.assume_init();
            Ok(find_data)
        }
    }
