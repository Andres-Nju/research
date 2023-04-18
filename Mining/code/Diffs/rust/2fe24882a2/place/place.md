File_Code/rust/2fe24882a2/place/place_after.rs --- Text (4 errors, exceeded DFT_PARSE_ERROR_LIMIT)
362             if self.layout.for_variant(bcx.ccx, variant_index).abi == layout::Abi::Uninhabited {                                                         362         if self.layout.for_variant(bcx.ccx, variant_index).abi == layout::Abi::Uninhabited {
363                 return;                                                                                                                                  363             return;
364             }                                                                                                                                            364         }
365             match self.layout.variants {                                                                                                                 365         match self.layout.variants {

