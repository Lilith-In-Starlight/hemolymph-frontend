use crate::Card;

pub fn fuzzy(card: &Card, query: &str) -> bool {
    card.description
        .to_lowercase()
        .contains(&query.to_lowercase())
        || card.name.to_lowercase().contains(&query.to_lowercase())
        || card.r#type.to_lowercase().contains(&query.to_lowercase())
        || card.kins.iter().any(|x| x.contains(&query.to_lowercase()))
        || card
            .keywords
            .iter()
            .any(|x| x.name.contains(&query.to_lowercase()))
}
