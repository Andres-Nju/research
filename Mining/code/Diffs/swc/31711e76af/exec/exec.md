File_Code/swc/31711e76af/exec/exec_after.rs --- Rust
10045     let src = r###"                                                                                                                                    10045     let src = r###"
10046         class Test {                                                                                                                                   10046         class Test {
10047             constructor(config) {                                                                                                                      10047             constructor(config) {
10048             const that = this;                                                                                                                         10048             const that = this;
10049             this.config = config;                                                                                                                      10049             this.config = config;
10050             this.options = {                                                                                                                           10050             this.options = {
10051                 config() {                                                                                                                             10051                 get config() {
10052                 return that.config;                                                                                                                    10052                     return that.config;
10053                 },                                                                                                                                     10053                 },
10054             };                                                                                                                                         10054             };
10055             }                                                                                                                                          10055             }
10056         }                                                                                                                                              10056         }
10057                                                                                                                                                        10057       
10058         const instance = new Test(42);                                                                                                                 10058         const instance = new Test(42);
10059         console.log(instance.options.config);                                                                                                          10059         console.log(instance.options.config); 
10060     "###;                                                                                                                                              10060     "###;

