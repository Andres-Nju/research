    fn parse(_: &ParserContext, input: &mut Parser) -> Result<Self, ()> {
        input.try(|i| LengthOrPercentage::parse_non_negative(i)).map(ShapeRadius::Length).or_else(|_| {
            match_ignore_ascii_case! { try!(input.expect_ident()),
                "closest-side" => Ok(ShapeRadius::ClosestSide),
                "farthest-side" => Ok(ShapeRadius::FarthestSide),
                _ => Err(())
            }
        })
    }
