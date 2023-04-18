fn features_enabled() -> Vec<String> {
    let mut names = vec!["default".to_string()];

    // NOTE: There should be another way to know features on.

    #[cfg(feature = "which-support")]
    {
        names.push("which".to_string());
    }

    // always include it?
    names.push("zip".to_string());

    #[cfg(feature = "trash-support")]
    {
        names.push("trash".to_string());
    }

    #[cfg(feature = "sqlite")]
    {
        names.push("sqlite".to_string());
    }

    #[cfg(feature = "dataframe")]
    {
        names.push("dataframe".to_string());
    }

    #[cfg(feature = "static-link-openssl")]
    {
        names.push("static-link-openssl".to_string());
    }

    #[cfg(feature = "extra")]
    {
        names.push("extra".to_string());
    }

    names.sort();

    names
}
