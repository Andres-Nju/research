File_Code/rust/fd868d4bf4/error_codes/error_codes_after.rs --- Rust
1899 E0623: r##"                                                                                                                                             1899 E0623: r##"
1900 A lifetime didn't match what was expected.                                                                                                              1900 A lifetime didn't match what was expected.
1901                                                                                                                                                         1901 
1902 Erroneous code example:                                                                                                                                 1902 Erroneous code example:
1903                                                                                                                                                         1903 
1904 ```compile_fail,E0623                                                                                                                                   1904 ```compile_fail,E0623
1905 struct Foo<'a> {                                                                                                                                        1905 struct Foo<'a> {
1906     x: &'a isize,                                                                                                                                       1906     x: &'a isize,
1907 }                                                                                                                                                       1907 }
1908                                                                                                                                                         1908 
1909 fn bar<'short, 'long>(c: Foo<'short>, l: &'long isize) {                                                                                                1909 fn bar<'short, 'long>(c: Foo<'short>, l: &'long isize) {
1910     let _: Foo<'long> = c; // error!                                                                                                                    1910     let _: Foo<'long> = c; // error!
1911 }                                                                                                                                                       1911 }
1912 ```                                                                                                                                                     1912 ```
1913                                                                                                                                                         1913 
1914 In this example, we tried to set a value with an incompatible lifetime to                                                                               1914 In this example, we tried to set a value with an incompatible lifetime to
1915 another one (`'long` is unrelated to `'short`). We can solve this issue in two different                                                                1915 another one (`'long` is unrelated to `'short`). We can solve this issue in
1916 ways:                                                                                                                                                   1916 two different ways:
1917                                                                                                                                                         1917 
1918 Either we make `'short` live at least as long as `'long`:                                                                                               1918 Either we make `'short` live at least as long as `'long`:
1919                                                                                                                                                         1919 
1920 ```                                                                                                                                                     1920 ```
1921 struct Foo<'a> {                                                                                                                                        1921 struct Foo<'a> {
1922     x: &'a isize,                                                                                                                                       1922     x: &'a isize,
1923 }                                                                                                                                                       1923 }
1924                                                                                                                                                         1924 
1925 // we set 'short to live at least as long as 'long                                                                                                      1925 // we set 'short to live at least as long as 'long
1926 fn bar<'short: 'long, 'long>(c: Foo<'short>, l: &'long isize) {                                                                                         1926 fn bar<'short: 'long, 'long>(c: Foo<'short>, l: &'long isize) {
1927     let _: Foo<'long> = c; // ok!                                                                                                                       1927     let _: Foo<'long> = c; // ok!
1928 }                                                                                                                                                       1928 }
1929 ```                                                                                                                                                     1929 ```
1930                                                                                                                                                         1930 
1931 Or we use only one lifetime:                                                                                                                            1931 Or we use only one lifetime:
1932                                                                                                                                                         1932 
1933 ```                                                                                                                                                     1933 ```
1934 struct Foo<'a> {                                                                                                                                        1934 struct Foo<'a> {
1935     x: &'a isize,                                                                                                                                       1935     x: &'a isize,
1936 }                                                                                                                                                       1936 }
1937                                                                                                                                                         1937 
1938 fn bar<'short>(c: Foo<'short>, l: &'short isize) {                                                                                                      1938 fn bar<'short>(c: Foo<'short>, l: &'short isize) {
1939     let _: Foo<'short> = c; // ok!                                                                                                                      1939     let _: Foo<'short> = c; // ok!
1940 }                                                                                                                                                       1940 }
1941 ```                                                                                                                                                     1941 ```
1942 "##,                                                                                                                                                    1942 "##,

