use hemoglobin::cards::Card;
use rand::seq::SliceRandom;
use reqwest::Client;
use yew::suspense::use_future;
use yew::{function_component, html, Html, HtmlResult, Properties};

use crate::app::{get_ascii_titlecase, get_filegarden_link, modify_title};
use crate::app::{HOST, PORT};

#[derive(Properties, PartialEq)]
#[allow(clippy::module_name_repetitions)]
pub struct CardDetailsProps {
    pub card_id: String,
}

enum CardDetailsErr {
    NotACard,
    BadResponse,
}

#[function_component(CardDetails)]
pub fn card_details(CardDetailsProps { card_id }: &CardDetailsProps) -> HtmlResult {
    let card_id = card_id.to_owned();
    let card = use_future(|| async move {
        let client = Client::new();
        let url = format!("http://{HOST}:{PORT}/api/card?id={}", card_id.clone());
        if let Ok(response) = client.get(&url).send().await {
            if let Ok(result) = response.json::<Card>().await {
                Ok(result)
            } else {
                Err(CardDetailsErr::NotACard)
            }
        } else {
            Err(CardDetailsErr::BadResponse)
        }
    })?;

    match *card {
        Err(CardDetailsErr::NotACard) => Ok(html! {
            <div>
                <p>{"Error: Server sent something that is not a card"}</p>
            </div>
        }),
        Err(CardDetailsErr::BadResponse) => Ok(html! {
            <div>
                <p>{"Error: Server couldn't be reached"}</p>
            </div>
        }),
        Ok(ref card) => {
            let name = &card.name;

            let description: Html = card
                .description
                .lines()
                .map(|line| {
                    html! {
                        <p class="description-line">{line}</p>
                    }
                })
                .collect();

            let r#type = &card.r#type;

            let img = &card.img;

            let img = img.choose(&mut rand::thread_rng()).unwrap_or(name);

            let cost = card.cost;
            let health = card.health;
            let defense = card.defense;
            let power = card.power;

            modify_title(name);

            Ok(html! {
                <div id="details-view">
                    <div id="details">
                        <img id="details-preview" src={get_filegarden_link(img)} />
                        <div id="text-description">
                            <h1 id="details-title">{name.clone()}</h1>
                            <hr />
                            <p id="cost-line">{get_ascii_titlecase(r#type)} if !r#type.contains("blood flask") {{" :: "} {cost} {" Blood"}}</p>
                            <hr />
                            {description}
                            if !r#type.contains("command") {
                                <hr />
                                <p id="stats-line">{health}{"/"}{defense}{"/"}{power}</p>
                            }
                        </div>
                    </div>
                </div>
            })
        }
    }
}
