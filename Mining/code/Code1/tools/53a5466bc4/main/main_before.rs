fn main() -> Result<()> {
    let root = project_root().join("website/docs/src/components");
    let mut args = Arguments::from_env();
    let token: String = args.value_from_str("--token").unwrap();
    let contributors = get_contributors(&token);

    let mut content = String::new();

    let command = "Use the command `cargo contributors`".to_string();
    write!(
        content,
        "{{/** {} */}}",
        prepend_generated_preamble(command)
    )?;
    content.push('\n');
    content.push_str("<h3>Code contributors</h3>");
    content.push('\n');
    content.push_str("<ul class=\"team-list contributors\">");
    for contributor in contributors {
        let mut contributor_html = String::new();
        let escaped_login = html_escape::encode_text(&contributor.login);
        let escaped_avatar = html_escape::encode_text(&contributor.avatar_url);
        contributor_html.push_str("<li><a href=\"https://github.com/rome/tools/commits?author=");

        html_escape::encode_double_quoted_attribute_to_string(
            format!("{}", escaped_login),
            &mut contributor_html,
        );
        contributor_html.push_str("\">");
        contributor_html.push_str("<img src=\"");
        html_escape::encode_double_quoted_attribute_to_string(
            format!("{}", escaped_avatar),
            &mut contributor_html,
        );
        content.push_str(&contributor_html);
        write!(content, "\" alt=\"{}\" />", contributor.login)?;
        write!(content, "<span>{}</span>", escaped_login)?;
        content.push_str("</a></li>");
    }

    content.push_str("</ul>");
    fs2::write(root.join("Contributors.astro"), content)?;

    Ok(())
}
