File_Code/yew/eaf5125018/lib/lib_after.rs --- 1/2 --- Rust
                                                                                                                                                             4 #[allow(unused_imports)]

File_Code/yew/eaf5125018/lib/lib_after.rs --- 2/2 --- Rust
59                 <textarea oninput=|input| Msg::Payload(input.value)                                                                                       60                 <textarea oninput=self.link.callback(move |input: InputData| Msg::Payload(input.value))

