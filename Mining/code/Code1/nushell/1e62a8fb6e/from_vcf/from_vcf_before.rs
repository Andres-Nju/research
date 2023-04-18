async fn from_vcf(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let args = args.evaluate_once(&registry).await?;
    let tag = args.name_tag();
    let input = args.input;

    let input_string = input.collect_string(tag.clone()).await?.item;
    let input_bytes = input_string.into_bytes();
    let buf_reader = std::io::Cursor::new(input_bytes);
    let parser = ical::VcardParser::new(buf_reader);

    let iter = parser.map(move |contact| match contact {
        Ok(c) => ReturnSuccess::value(contact_to_value(c, tag.clone())),
        Err(_) => Err(ShellError::labeled_error(
            "Could not parse as .vcf",
            "input cannot be parsed as .vcf",
            tag.clone(),
        )),
    });

    Ok(futures::stream::iter(iter).to_output_stream())
}
