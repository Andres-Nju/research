File_Code/ripgrep/3bb71b0cb8/app/app_after.rs --- 1/2 --- Rust
873         "\                                                                                                                                               873         "\
874 Print the 0-based byte offset within the input file before each line of output.                                                                          874 Print the 0-based byte offset within the input file before each line of output.
875 If -o (--only-matching) is specified, print the offset of the matching part                                                                              875 If -o (--only-matching) is specified, print the offset of the matching part
876 itself.                                                                                                                                                  876 itself.
877                                                                                                                                                          877 
878 If ripgrep does transcoding, then the byte offset is in terms of the the result                                                                          878 If ripgrep does transcoding, then the byte offset is in terms of the result of
879 of transcoding and not the original data. This applies similarly to another                                                                              879 transcoding and not the original data. This applies similarly to another
880 transformation on the source, such as decompression or a --pre filter. Note                                                                              880 transformation on the source, such as decompression or a --pre filter. Note
881 that when the PCRE2 regex engine is used, then UTF-8 transcoding is done by                                                                              881 that when the PCRE2 regex engine is used, then UTF-8 transcoding is done by
882 default.                                                                                                                                                 882 default.
883 "                                                                                                                                                        883 "

File_Code/ripgrep/3bb71b0cb8/app/app_after.rs --- 2/2 --- Rust
942         "\                                                                                                                                               942         "\
943 This flag specifies color settings for use in the output. This flag may be                                                                               943 This flag specifies color settings for use in the output. This flag may be
944 provided multiple times. Settings are applied iteratively. Colors are limited                                                                            944 provided multiple times. Settings are applied iteratively. Colors are limited
945 to one of eight choices: red, blue, green, cyan, magenta, yellow, white and                                                                              945 to one of eight choices: red, blue, green, cyan, magenta, yellow, white and
946 black. Styles are limited to nobold, bold, nointense, intense, nounderline                                                                               946 black. Styles are limited to nobold, bold, nointense, intense, nounderline
947 or underline.                                                                                                                                            947 or underline.
948                                                                                                                                                          948 
949 The format of the flag is '{type}:{attribute}:{value}'. '{type}' should be                                                                               949 The format of the flag is '{type}:{attribute}:{value}'. '{type}' should be
950 one of path, line, column or match. '{attribute}' can be fg, bg or style.                                                                                950 one of path, line, column or match. '{attribute}' can be fg, bg or style.
951 '{value}' is either a color (for fg and bg) or a text style. A special format,                                                                           951 '{value}' is either a color (for fg and bg) or a text style. A special format,
952 '{type}:none', will clear all color settings for '{type}'.                                                                                               952 '{type}:none', will clear all color settings for '{type}'.
953                                                                                                                                                          953 
954 For example, the following command will change the match color to magenta and                                                                            954 For example, the following command will change the match color to magenta and
955 the background color for line numbers to yellow:                                                                                                         955 the background color for line numbers to yellow:
956                                                                                                                                                          956 
957     rg --colors 'match:fg:magenta' --colors 'line:bg:yellow' foo.                                                                                        957     rg --colors 'match:fg:magenta' --colors 'line:bg:yellow' foo.
958                                                                                                                                                          958 
959 Extended colors can be used for '{value}' when the terminal supports ANSI color                                                                          959 Extended colors can be used for '{value}' when the terminal supports ANSI color
960 sequences. These are specified as either 'x' (256-color) or 'x,x,x' (24-bit                                                                              960 sequences. These are specified as either 'x' (256-color) or 'x,x,x' (24-bit
961 truecolor) where x is a number between 0 and 255 inclusive. x may be given as                                                                            961 truecolor) where x is a number between 0 and 255 inclusive. x may be given as
962 a normal decimal number or a hexadecimal number, which is prefixed by `0x`.                                                                              962 a normal decimal number or a hexadecimal number, which is prefixed by `0x`.
963                                                                                                                                                          963 
964 For example, the following command will change the match background color to                                                                             964 For example, the following command will change the match background color to
965 that represented by the rgb value (0,128,255):                                                                                                           965 that represented by the rgb value (0,128,255):
966                                                                                                                                                          966 
967     rg --colors 'match:bg:0,128,255'                                                                                                                     967     rg --colors 'match:bg:0,128,255'
968                                                                                                                                                          968 
969 or, equivalently,                                                                                                                                        969 or, equivalently,
970                                                                                                                                                          970 
971     rg --colors 'match:bg:0x0,0x80,0xFF'                                                                                                                 971     rg --colors 'match:bg:0x0,0x80,0xFF'
972                                                                                                                                                          972 
973 Note that the the intense and nointense style flags will have no effect when                                                                             973 Note that the intense and nointense style flags will have no effect when
974 used alongside these extended color codes.                                                                                                               974 used alongside these extended color codes.
975 "                                                                                                                                                        975 "

