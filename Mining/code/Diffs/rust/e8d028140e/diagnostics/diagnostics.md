File_Code/rust/e8d028140e/diagnostics/diagnostics_after.rs --- Rust
1164 E0283: r##"                                                                                                                                             1164 E0283: r##"
1165 This error occurs when the compiler doesn't have enough information                                                                                     1165 This error occurs when the compiler doesn't have enough information
1166 to unambiguously choose an implementation.                                                                                                              1166 to unambiguously choose an implementation.
1167                                                                                                                                                         1167 
1168 For example:                                                                                                                                            1168 For example:
1169                                                                                                                                                         1169 
1170 ```compile_fail,E0283                                                                                                                                   1170 ```compile_fail,E0283
1171 trait Generator {                                                                                                                                       1171 trait Generator {
1172     fn create() -> u32;                                                                                                                                 1172     fn create() -> u32;
1173 }                                                                                                                                                       1173 }
1174                                                                                                                                                         1174 
1175 struct Impl;                                                                                                                                            1175 struct Impl;
1176                                                                                                                                                         1176 
1177 impl Generator for Impl {                                                                                                                               1177 impl Generator for Impl {
1178     fn create() -> u32 { 1 }                                                                                                                            1178     fn create() -> u32 { 1 }
1179 }                                                                                                                                                       1179 }
1180                                                                                                                                                         1180 
1181 struct AnotherImpl;                                                                                                                                     1181 struct AnotherImpl;
1182                                                                                                                                                         1182 
1183 impl Generator for AnotherImpl {                                                                                                                        1183 impl Generator for AnotherImpl {
1184     fn create() -> u32 { 2 }                                                                                                                            1184     fn create() -> u32 { 2 }
1185 }                                                                                                                                                       1185 }
1186                                                                                                                                                         1186 
1187 fn main() {                                                                                                                                             1187 fn main() {
1188     let cont: u32 = Generator::create();                                                                                                                1188     let cont: u32 = Generator::create();
1189     // error, impossible to choose one of Generator trait implementation                                                                                1189     // error, impossible to choose one of Generator trait implementation
1190     // Impl or AnotherImpl? Maybe anything else?                                                                                                        1190     // Should it be Impl or AnotherImpl, maybe something else?
1191 }                                                                                                                                                       1191 }
1192 ```                                                                                                                                                     1192 ```
1193                                                                                                                                                         1193 
1194 To resolve this error use the concrete type:                                                                                                            1194 To resolve this error use the concrete type:
1195                                                                                                                                                         1195 
1196 ```                                                                                                                                                     1196 ```
1197 trait Generator {                                                                                                                                       1197 trait Generator {
1198     fn create() -> u32;                                                                                                                                 1198     fn create() -> u32;
1199 }                                                                                                                                                       1199 }
1200                                                                                                                                                         1200 
1201 struct AnotherImpl;                                                                                                                                     1201 struct AnotherImpl;
1202                                                                                                                                                         1202 
1203 impl Generator for AnotherImpl {                                                                                                                        1203 impl Generator for AnotherImpl {
1204     fn create() -> u32 { 2 }                                                                                                                            1204     fn create() -> u32 { 2 }
1205 }                                                                                                                                                       1205 }
1206                                                                                                                                                         1206 
1207 fn main() {                                                                                                                                             1207 fn main() {
1208     let gen1 = AnotherImpl::create();                                                                                                                   1208     let gen1 = AnotherImpl::create();
1209                                                                                                                                                         1209 
1210     // if there are multiple methods with same name (different traits)                                                                                  1210     // if there are multiple methods with same name (different traits)
1211     let gen2 = <AnotherImpl as Generator>::create();                                                                                                    1211     let gen2 = <AnotherImpl as Generator>::create();
1212 }                                                                                                                                                       1212 }
1213 ```                                                                                                                                                     1213 ```
1214 "##,                                                                                                                                                    1214 "##,

