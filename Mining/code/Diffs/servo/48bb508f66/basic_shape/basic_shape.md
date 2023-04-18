File_Code/servo/48bb508f66/basic_shape/basic_shape_after.rs --- Rust
665     fn parse(context: &ParserContext, input: &mut Parser) -> Result<Self, ()> {                                                                          665     fn parse(_: &ParserContext, input: &mut Parser) -> Result<Self, ()> {
666         input.try(|i| LengthOrPercentage::parse(context, i)).map(ShapeRadius::Length).or_else(|_| {                                                      666         input.try(|i| LengthOrPercentage::parse_non_negative(i)).map(ShapeRadius::Length).or_else(|_| {

