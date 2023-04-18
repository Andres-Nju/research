File_Code/servo/70c62ceee7/http_loader/http_loader_after.rs --- Rust
1465     let is_same_origin = request.url_list.iter().any(|url| match request.origin {                                                                       1465     let is_same_origin = request.url_list.iter().all(|url| match request.origin {
1466         SpecificOrigin(ref immutable_request_origin) => {                                                                                               1466         SpecificOrigin(ref immutable_request_origin) => url.origin() == *immutable_request_origin,
1467             url.clone().into_url().origin().ascii_serialization() ==                                                                                         
1468                 immutable_request_origin.ascii_serialization()                                                                                               
1469         },                                                                                                                                                   

