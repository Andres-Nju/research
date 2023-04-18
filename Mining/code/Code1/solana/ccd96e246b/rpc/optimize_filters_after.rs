fn optimize_filters(filters: &mut [RpcFilterType]) {
    filters.iter_mut().for_each(|filter_type| {
        if let RpcFilterType::Memcmp(compare) = filter_type {
            use MemcmpEncodedBytes::*;
            match &compare.bytes {
                #[allow(deprecated)]
                Binary(bytes) | Base58(bytes) => {
                    if let Ok(bytes) = bs58::decode(bytes).into_vec() {
                        compare.bytes = Bytes(bytes);
                    }
                }
                Base64(bytes) => {
                    if let Ok(bytes) = base64::decode(bytes) {
                        compare.bytes = Bytes(bytes);
                    }
                }
                _ => {}
            }
        }
    })
}
