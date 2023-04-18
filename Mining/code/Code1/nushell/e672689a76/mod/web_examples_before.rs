pub fn web_examples() -> Vec<PluginExample> {
    vec![PluginExample {
        example: "http get https://phoronix.com | query web -q 'header'".into(),
        description: "Retrieve all <header> elements from phoronix.com website".into(),
        result: None,
    }, PluginExample {
        example: "http get https://en.wikipedia.org/wiki/List_of_cities_in_India_by_population
    | query web -t [Rank City 'Population(2011)[3]' 'Population(2001)' 'State or union territory']".into(),
        description: "Retrieve a html table from Wikipedia and parse it into a nushell table using table headers as guides".into(),
        result: None
    },
    PluginExample {
        example: "http get https://www.nushell.sh | query web -q 'h2, h2 + p' | group 2 | each {rotate --ccw tagline description} | flatten".into(),
        description: "Pass multiple css selectors to extract several elements within single query, group the query results together and rotate them to create a table".into(),
        result: None,
    },
    PluginExample {
        example: "http get https://example.org | query web --query a --attribute href".into(),
        description: "Retrieve a specific html attribute instead of the default text".into(),
        result: None,
    }]
}
