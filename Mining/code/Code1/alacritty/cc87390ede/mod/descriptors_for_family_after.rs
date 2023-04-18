pub fn descriptors_for_family(family: &str) -> Vec<Descriptor> {
    let mut out = Vec::new();

    info!("family: {}", family);
    let ct_collection = match create_for_family(family) {
        Some(c) => c,
        None => return out,
    };

    // CFArray of CTFontDescriptorRef (i think)
    let descriptors = ct_collection.get_descriptors();
    for descriptor in descriptors.iter() {
        out.push(Descriptor::new(descriptor.clone()));
    }

    out
}
