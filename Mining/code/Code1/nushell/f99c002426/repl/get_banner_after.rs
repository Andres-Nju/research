fn get_banner(engine_state: &mut EngineState, stack: &mut Stack) -> String {
    let age = match eval_string_with_input(
        engine_state,
        stack,
        None,
        "(date now) - ('2019-05-10 09:59:12-0700' | into datetime)",
    ) {
        Ok(Value::Duration { val, .. }) => format_duration(val),
        _ => "".to_string(),
    };

    let banner = format!(
        r#"{}     __  ,
{} .--()°'.' {}Welcome to {}Nushell{},
{}'|, . ,'   {}based on the {}nu{} language,
{} !_-(_\    {}where all data is structured!

Please join our {}Discord{} community at {}https://discord.gg/NtAbbGn{}
Our {}GitHub{} repository is at {}https://github.com/nushell/nushell{}
Our {}Documentation{} is located at {}http://nushell.sh{}
{}Tweet{} us at {}@nu_shell{}

It's been this long since {}Nushell{}'s first commit:
{}

{}You can disable this banner using the {}config nu{}{} command
to modify the config.nu file and setting show_banner to false.

let-env config = {{
    show_banner: false
    ...
}}{}
"#,
        "\x1b[32m",   //start line 1 green
        "\x1b[32m",   //start line 2
        "\x1b[0m",    //before welcome
        "\x1b[32m",   //before nushell
        "\x1b[0m",    //after nushell
        "\x1b[32m",   //start line 3
        "\x1b[0m",    //before based
        "\x1b[32m",   //before nu
        "\x1b[0m",    //after nu
        "\x1b[32m",   //start line 4
        "\x1b[0m",    //before where
        "\x1b[35m",   //before Discord purple
        "\x1b[0m",    //after Discord
        "\x1b[35m",   //before Discord URL
        "\x1b[0m",    //after Discord URL
        "\x1b[1;32m", //before GitHub green_bold
        "\x1b[0m",    //after GitHub
        "\x1b[1;32m", //before GitHub URL
        "\x1b[0m",    //after GitHub URL
        "\x1b[32m",   //before Documentation
        "\x1b[0m",    //after Documentation
        "\x1b[32m",   //before Documentation URL
        "\x1b[0m",    //after Documentation URL
        "\x1b[36m",   //before Tweet blue
        "\x1b[0m",    //after Tweet
        "\x1b[1;36m", //before @nu_shell cyan_bold
        "\x1b[0m",    //after @nu_shell
        "\x1b[32m",   //before Nushell
        "\x1b[0m",    //after Nushell
        age,
        "\x1b[2;37m", //before banner disable dim white
        "\x1b[2;36m", //before config nu dim cyan
        "\x1b[0m",    //after config nu
        "\x1b[2;37m", //after config nu dim white
        "\x1b[0m",    //after banner disable
    );

    banner
}
