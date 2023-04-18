File_Code/alacritty/5864c30a54/mod/mod_after.rs --- Rust
1568             *cell = self.cursor.template;                                                                                                               1568             if cell.c == ' ' {
1569             cell.c = self.cursor.charsets[self.active_charset].map('\t');                                                                               1569                 cell.c = self.cursor.charsets[self.active_charset].map('\t');
                                                                                                                                                             1570             }

