use rand::seq::SliceRandom;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/:query")]
    Search { query: String },
    #[at("/card/:id")]
    Card { id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Card {
    id: String,
    cost: usize,
    #[serde(default)]
    img: Vec<String>,
    name: String,
    health: usize,
    defense: usize,
    r#type: String,
    power: usize,
    description: String,
}

#[derive(Properties, PartialEq)]
struct CardDetailsProps {
    card_id: String,
}

#[derive(Properties, PartialEq)]
struct CardListProps {
    search: String,
}

#[derive(Deserialize, PartialEq)]
#[serde(tag = "type")]
enum QueryResult {
    CardList { content: Vec<Card> },
    Error { message: String },
}

#[function_component(CardList)]
fn card_list(CardListProps { search }: &CardListProps) -> Html {
    let result = use_state_eq(|| None);
    let search = search.clone();
    let result2 = result.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = result2.clone();
        let client = Client::new();
        let url = format!("http://127.0.0.1:3000/search?query={}", search.clone());
        if let Ok(response) = client.get(&url).send().await {
            match response.json::<QueryResult>().await {
                Ok(x) => result.set(Some(x)),
                Err(x) => panic!("{x:#?}"),
            }
        }
    });
    match result.as_ref() {
        Some(QueryResult::CardList { content }) => {
            let a = content
                .iter()
                .map(|card| {
                    html! {

                        <Link<Route> to={Route::Card{id: card.id.clone()}}><img class="card-result" src={get_filegarden_link(&card.img.choose(&mut rand::thread_rng()).unwrap_or(&card.name))} /></Link<Route>>
                    }
                });

            html! {
                <div id="results">
                    {for a}
                </div>
            }
        }
        Some(QueryResult::Error { message }) => {
            html! {
                <div id="search-error">
                    <p><b>{"ERROR:"}</b>{message}</p>
                </div>
            }
        }
        None => html! {
            <div id="results">
                {"Searching"}
            </div>
        },
    }
}

#[function_component(CardDetails)]
fn card_details(CardDetailsProps { card_id }: &CardDetailsProps) -> Html {
    let card = use_state_eq(|| None);
    let card_id = card_id.clone();
    let card2 = card.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let card = card2.clone();
        let client = Client::new();
        let url = format!("http://127.0.0.1:3000/card?id={}", card_id.clone());
        if let Ok(response) = client.get(&url).send().await {
            if let Ok(result) = response.json::<Card>().await {
                card.set(Some(result));
            }
        }
    });
    let name = card
        .as_ref()
        .map_or("ID not found".to_string(), |c| c.name.clone());

    let description: Html = card
        .as_ref()
        .map_or("ID not found".to_string(), |c| c.description.clone())
        .lines()
        .into_iter()
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

    let cost = card.as_ref().map_or(9999, |c| c.cost.clone());
    let health = card.as_ref().map_or(9999, |c| c.health.clone());
    let defense = card.as_ref().map_or(9999, |c| c.defense.clone());
    let power = card.as_ref().map_or(9999, |c| c.power.clone());

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
    let query = use_state(|| String::new());
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
            <Link<Route> to={Route::Search { query: "".to_string() }}><img id="logo" src="https://file.garden/ZJSEzoaUL3bz8vYK/hemolymphlogo.png" /></Link<Route>>
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
    yew::Renderer::<App>::new().render();
}
