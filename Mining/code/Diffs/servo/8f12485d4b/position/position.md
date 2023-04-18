File_Code/servo/8f12485d4b/position/position_after.rs --- 1/2 --- Text (256 errors, exceeded DFT_PARSE_ERROR_LIMIT)
498     /// Returns true if every sub property value of `grid` shorthand is initial.                                                                           . 
499     impl<'a> LonghandsToSerialize<'a> {                                                                                                                  498     impl<'a> LonghandsToSerialize<'a> {
500         fn is_initial(&self) -> bool {                                                                                                                   499         /// Returns true if other sub properties except template-{rows,columns} are initial.
501             *self.grid_template_rows == GridTemplateComponent::None &&                                                                                   ... 
502             *self.grid_template_columns == GridTemplateComponent::None &&                                                                                500         fn is_grid_template(&self) -> bool {

File_Code/servo/8f12485d4b/position/position_after.rs --- 2/2 --- Text (256 errors, exceeded DFT_PARSE_ERROR_LIMIT)
515                self.is_initial() {                                                                                                                       513                self.is_grid_template() {

