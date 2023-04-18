fn parse_content_distribution(input: &mut Parser) -> Result<AlignFlags, ()> {
    let ident = input.expect_ident()?;
    match_ignore_ascii_case! { &ident,
      "stretch" => Ok(ALIGN_STRETCH),
      "space-between" => Ok(ALIGN_SPACE_BETWEEN),
      "space-around" => Ok(ALIGN_SPACE_AROUND),
      "space-evenly" => Ok(ALIGN_SPACE_EVENLY),
      _ => Err(())
    }
}
