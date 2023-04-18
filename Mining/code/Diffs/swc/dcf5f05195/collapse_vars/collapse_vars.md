File_Code/swc/dcf5f05195/collapse_vars/collapse_vars_after.rs --- Text (1 error, exceeded DFT_PARSE_ERROR_LIMIT)
21             Expr::Assign(assign) => {                                                                                                                     21             Expr::Assign(assign @ AssignExpr { op: op!("="), .. }) => {

