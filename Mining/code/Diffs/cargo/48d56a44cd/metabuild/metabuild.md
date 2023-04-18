File_Code/cargo/48d56a44cd/metabuild/metabuild_after.rs --- 1/2 --- Rust
703             r#"                                                                                                                                          703             r#"
704 {                                                                                                                                                        704 {
705   "executable": null,                                                                                                                                    705   "executable": null,
706   "features": [],                                                                                                                                        706   "features": [],
707   "filenames": [                                                                                                                                         707   "filenames": [
708     "[..]/foo/target/debug/build/foo-[..]/metabuild-foo[EXE]"                                                                                            708     "[..]/foo/target/debug/build/foo-[..]/metabuild-foo[EXE]"
709   ],                                                                                                                                                     709   ],
710   "fresh": false,                                                                                                                                        710   "fresh": false,
711   "package_id": "foo [..]",                                                                                                                              711   "package_id": "foo [..]",
712   "profile": "{...}",                                                                                                                                    712   "profile": "{...}",
713   "reason": "compiler-artifact",                                                                                                                         713   "reason": "compiler-artifact",
714   "target": {                                                                                                                                            714   "target": {
715     "crate_types": [                                                                                                                                     715     "crate_types": [
716       "bin"                                                                                                                                              716       "bin"
717     ],                                                                                                                                                   717     ],
718     "edition": "2015",                                                                                                                                   718     "edition": "2018",
719     "kind": [                                                                                                                                            719     "kind": [
720       "custom-build"                                                                                                                                     720       "custom-build"
721     ],                                                                                                                                                   721     ],
722     "name": "metabuild-foo",                                                                                                                             722     "name": "metabuild-foo",
723     "src_path": "[..]/foo/target/.metabuild/metabuild-foo-[..].rs"                                                                                       723     "src_path": "[..]/foo/target/.metabuild/metabuild-foo-[..].rs"
724   }                                                                                                                                                      724   }
725 }                                                                                                                                                        725 }
726                                                                                                                                                          726 
727 {                                                                                                                                                        727 {
728   "cfgs": [],                                                                                                                                            728   "cfgs": [],
729   "env": [],                                                                                                                                             729   "env": [],
730   "linked_libs": [],                                                                                                                                     730   "linked_libs": [],
731   "linked_paths": [],                                                                                                                                    731   "linked_paths": [],
732   "package_id": "foo [..]",                                                                                                                              732   "package_id": "foo [..]",
733   "reason": "build-script-executed"                                                                                                                      733   "reason": "build-script-executed"
734 }                                                                                                                                                        734 }
735 "#,                                                                                                                                                      735 "#,

File_Code/cargo/48d56a44cd/metabuild/metabuild_after.rs --- 2/2 --- Rust
749             r#"                                                                                                                                          749             r#"
750 {                                                                                                                                                        750 {
751   "message": {                                                                                                                                           751   "message": {
752     "children": "{...}",                                                                                                                                 752     "children": "{...}",
753     "code": "{...}",                                                                                                                                     753     "code": "{...}",
754     "level": "error",                                                                                                                                    754     "level": "error",
755     "message": "cannot find function `metabuild` in module `mb`",                                                                                        755     "message": "cannot find function `metabuild` in module `mb`",
756     "rendered": "[..]",                                                                                                                                  756     "rendered": "[..]",
757     "spans": "{...}"                                                                                                                                     757     "spans": "{...}"
758   },                                                                                                                                                     758   },
759   "package_id": "foo [..]",                                                                                                                              759   "package_id": "foo [..]",
760   "reason": "compiler-message",                                                                                                                          760   "reason": "compiler-message",
761   "target": {                                                                                                                                            761   "target": {
762     "crate_types": [                                                                                                                                     762     "crate_types": [
763       "bin"                                                                                                                                              763       "bin"
764     ],                                                                                                                                                   764     ],
765     "edition": "2015",                                                                                                                                   765     "edition": "2018",
766     "kind": [                                                                                                                                            766     "kind": [
767       "custom-build"                                                                                                                                     767       "custom-build"
768     ],                                                                                                                                                   768     ],
769     "name": "metabuild-foo",                                                                                                                             769     "name": "metabuild-foo",
770     "src_path": null                                                                                                                                     770     "src_path": null
771   }                                                                                                                                                      771   }
772 }                                                                                                                                                        772 }
773 "#,                                                                                                                                                      773 "#,

