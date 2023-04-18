File_Code/rust/e9d70417ca/error/error_after.rs --- 1/2 --- Text (4 errors, exceeded DFT_PARSE_ERROR_LIMIT)
295     /// let error = io::Error::from_raw_os_error(98);                                                                                                    295     /// let error = io::Error::from_raw_os_error(22);
296     /// assert_eq!(error.kind(), io::ErrorKind::AddrInUse);                                                                                              296     /// assert_eq!(error.kind(), io::ErrorKind::InvalidInput);

File_Code/rust/e9d70417ca/error/error_after.rs --- 2/2 --- Text (4 errors, exceeded DFT_PARSE_ERROR_LIMIT)
306     /// let error = io::Error::from_raw_os_error(10048);                                                                                                 306     /// let error = io::Error::from_raw_os_error(10022);
307     /// assert_eq!(error.kind(), io::ErrorKind::AddrInUse);                                                                                              307     /// assert_eq!(error.kind(), io::ErrorKind::InvalidInput);

