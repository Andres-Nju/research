fn optimize_filters(filters: &mut [RpcFilterType]) {
    filters.iter_mut().for_each(|filter_type| {
        if let RpcFilterType::Memcmp(compare) = filter_type {
            use MemcmpEncodedBytes::*;
            match &compare.bytes {
                #[allow(deprecated)]
                Binary(bytes) | Base58(bytes) => {
                    compare.bytes = Bytes(bs58::decode(bytes).into_vec().unwrap());
                }
                Base64(bytes) => {
                    compare.bytes = Bytes(base64::decode(bytes).unwrap());
                }
                _ => {}
            }
        }
    })
}
