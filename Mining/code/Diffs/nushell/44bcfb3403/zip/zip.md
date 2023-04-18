File_Code/nushell/44bcfb3403/zip/zip_after.rs --- Rust
 5 const ZIP_POWERED_TEST_ASSERTION_SCRIPT: &str = r#"                                                                                                        5 const ZIP_POWERED_TEST_ASSERTION_SCRIPT: &str = r#"
 6 def expect [                                                                                                                                               6 def expect [
 7     left,                                                                                                                                                  7     left,
 8     --to-eq,                                                                                                                                               8     --to-eq,
 9     right                                                                                                                                                  9     right
10 ] {                                                                                                                                                       10 ] {
11     $left | zip { $right } | all? {|row|                                                                                                                  11     $left | zip $right | all? {|row|
12         $row.name.0 == $row.name.1 && $row.commits.0 == $row.commits.1                                                                                    12         $row.name.0 == $row.name.1 && $row.commits.0 == $row.commits.1
13     }                                                                                                                                                     13     }
14 }                                                                                                                                                         14 }
15 "#;                                                                                                                                                       15 "#;
16                                                                                                                                                           16 
17 // FIXME: jt: needs more work                                                                                                                                
18 #[ignore]                                                                                                                                                    

