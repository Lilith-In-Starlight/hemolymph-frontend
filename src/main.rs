mod search;
use std::sync::Mutex;

use hemoglobin::{
    cards::Card,
    search::{query_parser::query_parser, QueryRestriction},
};
use rand::seq::SliceRandom;
use reqwest::Client;
use rust_fuzzy_search::fuzzy_compare;
use yew::prelude::*;
use yew_router::prelude::*;

static CARDS_JSON: Mutex<Vec<Card>> = Mutex::new(vec![]);

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/:query")]
    Search { query: String },
    #[at("/card/:id")]
    Card { id: String },
}

#[derive(Properties, PartialEq)]
struct CardDetailsProps {
    card_id: String,
}

#[derive(Properties, PartialEq)]
struct CardListProps {
    search: String,
}

#[derive(PartialEq)]
enum QueryResult<'a> {
    CardList { content: Vec<&'a Card> },
    Error { message: String },
}

pub enum Errors {
    InvalidComparisonString,
    UnknownParam,
}

#[function_component(CardList)]
fn card_list(CardListProps { search }: &CardListProps) -> Html {
    let cards = CARDS_JSON.lock().unwrap();
    let search = search.clone();
    let results = {
        match query_parser(&search) {
            Ok(query_restrictions) => {
                let mut name = String::new();
                let mut found_cards: Vec<&Card> = cards
                    .iter()
                    .filter(|card| {
                        let mut filtered = true;
                        for res in &query_restrictions {
                            match res {
                                QueryRestriction::Fuzzy(x) => {
                                    filtered = filtered && search::fuzzy(card, x);
                                    name = x.clone();
                                }
                                QueryRestriction::Comparison(field, comparison) => {
                                    filtered = filtered && comparison.compare(&field(card));
                                }
                                QueryRestriction::Contains(what, contains) => {
                                    filtered = filtered
                                        && what(card)
                                            .to_lowercase()
                                            .contains(contains.to_lowercase().as_str());
                                }
                                QueryRestriction::Has(fun, thing) => {
                                    let x = fun(card);
                                    filtered = filtered && x.iter().any(|x| x.contains(thing));
                                }
                                QueryRestriction::HasKw(fun, thing) => {
                                    let x = fun(card);
                                    filtered = filtered && x.iter().any(|x| x.name.contains(thing));
                                }
                            }
                        }
                        filtered
                    })
                    .collect();

                found_cards.sort_by(|a, b| {
                    weighted_compare(b, &name)
                        .partial_cmp(&weighted_compare(a, &name))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                QueryResult::CardList {
                    content: found_cards,
                }
            }
            Err(_) => QueryResult::Error {
                message: "Query couldn't be parsed".to_string(),
            },
        }
    };

    match results {
        QueryResult::CardList { content } => {
            let a = content
                .iter()
                .map(|card| {
                    html! {

                        <Link<Route> to={Route::Card{id: card.id.clone()}}><img class="card-result" src={get_filegarden_link(card.img.choose(&mut rand::thread_rng()).unwrap_or(&card.name))} /></Link<Route>>
                    }
                });

            html! {
                <div id="results">
                    {for a}
                </div>
            }
        }
        QueryResult::Error { message } => {
            html! {
                <div id="search-error">
                    <p><b>{"ERROR:"}</b>{message}</p>
                </div>
            }
        }
    }
}

fn weighted_compare(a: &Card, b: &str) -> f32 {
    fuzzy_compare(&a.name, b) * 2.
        + fuzzy_compare(&a.r#type, b) * 1.8
        + fuzzy_compare(&a.description, b) * 1.6
        + a.kins
            .iter()
            .map(|x| fuzzy_compare(x, b))
            .max_by(|a, b| PartialOrd::partial_cmp(a, b).unwrap_or(std::cmp::Ordering::Less))
            .unwrap_or(0.0)
            * 1.5
        + a.keywords
            .iter()
            .map(|x| fuzzy_compare(&x.name, b))
            .max_by(|a, b| PartialOrd::partial_cmp(a, b).unwrap_or(std::cmp::Ordering::Less))
            .unwrap_or(0.0)
            * 1.2
}
#[function_component(CardDetails)]
fn card_details(CardDetailsProps { card_id }: &CardDetailsProps) -> Html {
    let cardjson = CARDS_JSON.lock().unwrap();
    let card = cardjson
        .iter()
        .filter(|x| x.id == *card_id)
        .collect::<Vec<&Card>>();
    let card = card.first();

    let name = card
        .as_ref()
        .map_or("ID not found".to_string(), |c| c.name.clone());

    let description: Html = card
        .as_ref()
        .map_or("ID not found".to_string(), |c| c.description.clone())
        .lines()
        .map(|line| {
            html! {
                <p class="description-line">{line}</p>
            }
        })
        .collect();

    let r#type: String = card
        .as_ref()
        .map_or("ID not found".to_string(), |c| c.r#type.clone());

    let img = card.as_ref().map_or(vec![], |c| c.img.clone());

    let img = img.choose(&mut rand::thread_rng()).unwrap_or(&name);

    let cost = card.as_ref().map_or(9999, |c| c.cost);
    let health = card.as_ref().map_or(9999, |c| c.health);
    let defense = card.as_ref().map_or(9999, |c| c.defense);
    let power = card.as_ref().map_or(9999, |c| c.power);

    html! {
        <div id="details">
            <img id="details-preview" src={get_filegarden_link(img)} />
            <div id="text-description">
                <h1 id="details-title">{name.clone()}</h1>
                <hr />
                <p id="cost-line">{get_ascii_titlecase(&r#type)} {" :: "} {cost} {" Blood"}</p>
                <hr />
                {description}
                <hr />
                <p id="stats-line">{health}{"/"}{defense}{"/"}{power}</p>
            </div>
        </div>
    }
}

fn get_ascii_titlecase(s: &str) -> String {
    let mut b = s.to_string();
    if let Some(r) = b.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
    b
}

#[function_component(SearchBar)]
fn search_bar() -> Html {
    let nav = use_navigator().unwrap();
    let query = use_state(String::new);
    let oninput = {
        let query = query.clone();
        Callback::from(move |e: InputEvent| {
            let nav = nav.clone();
            let input = e
                .target_unchecked_into::<web_sys::HtmlInputElement>()
                .value();
            query.set(input.clone());
            nav.replace(&Route::Search {
                query: input.clone(),
            })
        })
    };

    html! {
         <nav id="search">
            <Link<Route> to={Route::Search { query: (*query).clone() }}><img id="logo" src="https://file.garden/ZJSEzoaUL3bz8vYK/hemolymphlogo.png" /></Link<Route>>
            <input id="search-bar" type="text" value={(*query).clone()} {oninput} />
        </nav>
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <SearchBar />
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

fn switch(route: Route) -> Html {
    match route {
        Route::Search { query } => html! {<CardList search={query} />},
        Route::Card { id } => html! {<CardDetails card_id={id}/>},
    }
}

fn get_filegarden_link(name: &str) -> String {
    format!(
        "https://file.garden/ZJSEzoaUL3bz8vYK/bloodlesscards/{}.png",
        name.replace(" ", "").replace("ä", "a")
    )
}

pub fn main() {
    wasm_bindgen_futures::spawn_local(async move {
        let mut cards_json = CARDS_JSON.lock().unwrap();
        let client = Client::new();
        let url = "https://ampersandia.net/bloodless/cards.json";
        let a = client.get(url).send().await.unwrap();
        let mut b = a.json::<Vec<Card>>().await.unwrap();
        cards_json.append(&mut b);
    });
    yew::Renderer::<App>::new().render();
}
