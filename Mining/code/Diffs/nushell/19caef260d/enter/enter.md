File_Code/nushell/19caef260d/enter/enter_after.rs --- 1/2 --- Rust
40         if !new_path.exists() {                                                                                                                             
41             return Err(ShellError::DirectoryNotFound(path_span));                                                                                           
42         }                                                                                                                                                   
43                                                                                                                                                             
44         if !new_path.is_dir() {                                                                                                                             
45             return Err(ShellError::DirectoryNotFoundCustom(                                                                                                 
46                 "not a directory".to_string(),                                                                                                              
47                 path_span,                                                                                                                                  
48             ));                                                                                                                                             
49         }                                                                                                                                                   

File_Code/nushell/19caef260d/enter/enter_after.rs --- 2/2 --- Rust
                                                                                                                                                             44         if !new_path.exists() {
                                                                                                                                                             45             return Err(ShellError::DirectoryNotFound(path_span));
                                                                                                                                                             46         }
                                                                                                                                                             47 
                                                                                                                                                             48         if !new_path.is_dir() {
                                                                                                                                                             49             return Err(ShellError::DirectoryNotFoundCustom(
                                                                                                                                                             50                 "not a directory".to_string(),
                                                                                                                                                             51                 path_span,
                                                                                                                                                             52             ));
                                                                                                                                                             53         }

