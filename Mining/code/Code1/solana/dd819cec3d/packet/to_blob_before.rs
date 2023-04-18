pub fn to_blob<T: Serialize>(
    resp: T,
    rsp_addr: SocketAddr,
    blob_recycler: &BlobRecycler,
) -> Result<SharedBlob> {
    let blob = blob_recycler.allocate();
    {
        let mut b = blob.write().unwrap();
        let v = serialize(&resp)?;
        let len = v.len();
        assert!(len < BLOB_SIZE);
        b.data[..len].copy_from_slice(&v);
        b.meta.size = len;
        b.meta.set_addr(&rsp_addr);
    }
    Ok(blob)
}
