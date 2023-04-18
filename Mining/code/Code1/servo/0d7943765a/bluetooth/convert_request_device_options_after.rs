fn convert_request_device_options(options: &RequestDeviceOptions,
                                  global: GlobalRef)
                                  -> Fallible<RequestDeviceoptions> {
    if options.filters.is_empty() {
        return Err(Type(FILTER_EMPTY_ERROR.to_owned()));
    }

    let mut filters = vec!();
    for filter in &options.filters {
        filters.push(try!(canonicalize_filter(&filter, global)));
    }

    let mut optional_services = vec!();
    if let Some(ref opt_services) = options.optionalServices {
        for opt_service in opt_services {
            let uuid = try!(BluetoothUUID::GetService(global, opt_service.clone())).to_string();
            if !uuid_is_blacklisted(uuid.as_ref(), Blacklist::All) {
                optional_services.push(uuid);
            }
        }
    }

    Ok(RequestDeviceoptions::new(BluetoothScanfilterSequence::new(filters),
                                 ServiceUUIDSequence::new(optional_services)))
}
