File_Code/rust/dc54cd0c60/validity/validity_after.rs --- 1/3 --- Rust
69     GeneratoreState(VariantIdx),                                                                                                                          69     GeneratorState(VariantIdx),

File_Code/rust/dc54cd0c60/validity/validity_after.rs --- 2/3 --- Rust
104             GeneratoreState(idx) => write!(out, ".<generator-state({})>", idx.index()),                                                                  104             GeneratorState(idx) => write!(out, ".<generator-state({})>", idx.index()),

File_Code/rust/dc54cd0c60/validity/validity_after.rs --- 3/3 --- Rust
270             ty::Generator(..) => PathElem::GeneratoreState(variant_id),                                                                                  270             ty::Generator(..) => PathElem::GeneratorState(variant_id),

