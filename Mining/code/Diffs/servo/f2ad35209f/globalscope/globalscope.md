File_Code/servo/f2ad35209f/globalscope/globalscope_after.rs --- 1/2 --- Rust
303         if let Some(worker) = self.downcast::<WorkletGlobalScope>() {                                                                                    303         if let Some(worklet) = self.downcast::<WorkletGlobalScope>() {
304             // https://drafts.css-houdini.org/worklets/#script-settings-for-worklets                                                                     304             // https://drafts.css-houdini.org/worklets/#script-settings-for-worklets
305             return worker.base_url();                                                                                                                    305             return worklet.base_url();

File_Code/servo/f2ad35209f/globalscope/globalscope_after.rs --- 2/2 --- Rust
318         if let Some(worker) = self.downcast::<WorkletGlobalScope>() {                                                                                    318         if let Some(worklet) = self.downcast::<WorkletGlobalScope>() {
319             // TODO: is this the right URL to return?                                                                                                    319             // TODO: is this the right URL to return?
320             return worker.base_url();                                                                                                                    320             return worklet.base_url();

