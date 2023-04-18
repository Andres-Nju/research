File_Code/nushell/35b12fe5ec/rm/rm_after.rs --- Rust
129             example: "ls | where size == 0KB && type == file | each { rm $in.name } | null",                                                             129             example: "ls | where size == 0KB and type == file | each { rm $in.name } | null",

