pub fn rename_all(style: &str, ident: &str) -> Option<String> {
    match style {
        "lowercase" => Some(ident.to_ascii_lowercase()),
        "UPPERCASE" => Some(ident.to_ascii_uppercase()),
        "PascalCase" => Some(pascal(ident)),
        "camelCase" => Some(camel(ident)),
        "snake_case" => Some(words(ident).join("_")),
        "SCREAMING_SNAKE_CASE" => Some(words(ident).join("_").to_ascii_uppercase()),
        "kebab-case" => Some(words(ident).join("-")),
        "SCREAMING-KEBAB-CASE" => Some(words(ident).join("-").to_ascii_uppercase()),
        _ => None,
    }
}

fn pascal(ident: &str) -> String {
    words(ident)
        .into_iter()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn camel(ident: &str) -> String {
    let mut pascal = pascal(ident);
    if let Some(first) = pascal.get_mut(0..1) {
        first.make_ascii_lowercase();
    }
    pascal
}

fn words(ident: &str) -> Vec<String> {
    ident
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}
