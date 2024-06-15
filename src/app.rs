#![warn(clippy::pedantic)]
use std::collections::HashMap;
use std::sync::Mutex;

use gloo_timers::callback::Timeout;
use hemoglobin::cards::Card;
use rand::seq::SliceRandom;
use reqwest::Client;
use serde::Deserialize;
use web_sys::wasm_bindgen::JsCast;
use web_sys::HtmlMetaElement;
use yew::prelude::*;
use yew_router::history::AnyHistory;
use yew_router::history::History;
use yew_router::history::MemoryHistory;
use yew_router::prelude::*;
use yew_router::Router;

static QUERY: Mutex<String> = Mutex::new(String::new());
#[cfg(not(debug_assertions))]
static HOST: &'static str = "104.248.54.50";
#[cfg(not(debug_assertions))]
static PORT: &'static str = "80";

#[cfg(debug_assertions)]
static HOST: &str = "127.0.0.1";
#[cfg(debug_assertions)]
static PORT: &str = "8080";

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/:query")]
    Search { query: String },
    #[at("/card/:id")]
    Card { id: String },
    #[at("/howto")]
    Instructions,
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
    if search.trim().is_empty() {
        modify_title("");
    } else {
        modify_title("Searching");
    }
    let result = use_state_eq(|| None);
    let search = search.clone();
    let result2 = result.clone();
    use_effect(move || {
        run_future(async move {
            let result = result2.clone();
            let client = Client::new();
            let url = format!("http://{HOST}:{PORT}/api/search?query={}", search.clone());
            match client.get(&url).send().await {
                Ok(response) => match response.json::<QueryResult>().await {
                    Ok(queryres) => result.set(Some(queryres)),
                    Err(err) => result.set(Some(QueryResult::Error {
                        message: format!("Obtained a malformed response: \n{err}"),
                    })),
                },
                Err(err) => result.set(Some(QueryResult::Error {
                    message: format!("Couldn't get a response from the server. {err}"),
                })),
            }
        });
    });
    match result.as_ref() {
        Some(QueryResult::CardList { content }) => {
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
    use_effect(move || {
        run_future(async move {
            let card = card2.clone();
            let client = Client::new();
            let url = format!("http://{HOST}:{PORT}/api/card?id={}", card_id.clone());
            if let Ok(response) = client.get(&url).send().await {
                if let Ok(result) = response.json::<Card>().await {
                    card.set(Some(result));
                }
            }
        });
    });
    let card = card.as_ref();
    let name = card.map_or("ID not found".to_string(), |c| c.name.clone());

    let description: Html = card
        .map_or("ID not found".to_string(), |c| c.description.clone())
        .lines()
        .map(|line| {
            html! {
                <p class="description-line">{line}</p>
            }
        })
        .collect();

    let r#type: String = card.map_or("ID not found".to_string(), |c| c.r#type.clone());

    let img = card.as_ref().map_or(vec![], |c| c.img.clone());

    let img = img.choose(&mut rand::thread_rng()).unwrap_or(&name);

    let cost = card.map_or(9999, |c| c.cost);
    let health = card.map_or(9999, |c| c.health);
    let defense = card.map_or(9999, |c| c.defense);
    let power = card.map_or(9999, |c| c.power);

    modify_title(card.map_or("ID not found", |x| &x.name));

    html! {
        <div id="details-view">
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
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
fn modify_meta_tag_content(name: &str, new_content: &str) {
    let window = web_sys::window().expect("No window exists");
    let document = window.document().expect("No document on window");
    let desc = document
        .query_selector(&format!("meta[name=\"{name}\"]"))
        .expect("Couldn't find meta element")
        .expect("Couldn't find meta element")
        .dyn_into::<HtmlMetaElement>()
        .unwrap();

    desc.set_content(new_content);
}
#[cfg(target_arch = "wasm32")]
fn modify_title(title: &str) {
    let title = title.trim();
    let window = web_sys::window().expect("No window exists");
    let document = window.document().expect("No document on window");
    if title.is_empty() {
        document.set_title("Hemolymph");
    } else {
        document.set_title(&format!("{title} - Hemolymph"));
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn modify_title(title: &str) {}

#[cfg(not(target_arch = "wasm32"))]
fn modify_meta_tag_content(name: &str, new_content: &str) {}

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
    let state = use_state(|| false);
    let debounce_task = use_mut_ref::<Option<Timeout>, _>(|| None);
    let oninput = {
        Callback::from(move |e: InputEvent| {
            let state = state.clone();
            let nav = nav.clone();
            if let Some(task) = debounce_task.borrow_mut().take() {
                task.cancel();
            }
            let task = Timeout::new(500, move || {
                let mut query = QUERY.lock().unwrap();
                let input = e
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value();
                state.set(!*state);
                query.clone_from(&input);
                nav.replace(&Route::Search {
                    query: input.clone(),
                });
            });

            debounce_task.borrow_mut().replace(task);
        })
    };

    let quer = QUERY.lock().unwrap().clone();

    html! {
         <nav id="search">
            <Link<Route> to={Route::Search { query: String::new() }}><img id="logo" src="https://file.garden/ZJSEzoaUL3bz8vYK/hemolymphlogo.png" /></Link<Route>>
            <input id="search-bar" type="text" value={quer.clone()} {oninput} />
            <Link<Route> to={Route::Search {query: quer}}><span>{"Back to search"}</span></Link<Route>>
            <Link<Route> to={Route::Instructions}><span>{"How To Use"}</span></Link<Route>>
        </nav>
    }
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <BrowserRouter>
            <SearchBar />
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

#[derive(Properties, PartialEq, Eq, Debug)]
pub struct ServerAppProps {
    pub url: AttrValue,
    pub queries: HashMap<String, String>,
}

#[function_component(ServerApp)]
pub fn server_app(props: &ServerAppProps) -> Html {
    let history = AnyHistory::from(MemoryHistory::new());
    history
        .push_with_query(&*props.url, &props.queries)
        .unwrap();
    html! {
        <Router history={history}>
            <SearchBar />
            <Switch<Route> render={switch} />
        </Router>
    }
}

fn switch(route: Route) -> Html {
    match route {
        Route::Search { query } => html! {<CardList search={query} />},
        Route::Card { id } => html! {<CardDetails card_id={id}/>},
        Route::Instructions => {
            modify_title("How To");
            html! {
                <div id="instructions">
                    <section>
                        <h2>{"How to use Hemolymph"}</h2>
                        <p>{"Hemolymph is the arthropod equivalent of blood. It is also Bloodless' official card database."}</p>
                        <section class="instruction">
                            <h3>{"Fuzzy Search"}</h3>
                            <p>{"By default, your searches look for matches in Name, Kins, Keywords and Description, prioritizing them in that order."}</p>
                        </section>
                        <section class="instruction">
                            <h3>{"Name"}</h3>
                            <p>{"If you want to search by name only, you can write "}<span class="code">{"name:"}</span>{" or "}<span class="code">{"n:"}</span>{" before the name."}</p>
                        </section>
                        <section class="instruction">
                            <h3>{"Kins, Types and Keywords"}</h3>
                            <p>{"You can use "}<span class="code">{"k:"}</span>{" for kins and "}<span class="code">{"kw:"}</span>{" for kin. If you want to match more than one kin, they have to be separate. To search by type, use "} <span class="code">{"t:"}</span>{"."}</p>
                        </section>
                        <section class="Stats">
                            <h3>{"Kins and Keywords"}</h3>
                            <p>{"You can use "}<span class="code">{"h: d: p:"}</span>{" and "}<span class="code">{"c:"}</span>{" for health, defense, power and strength, respectively. You can also match comparisons, like "}<span class="code">{"c<=1 h=2 d>1 p!=2"}</span>{"."}</p>
                        </section>
                        <section class="Function">
                            <h3>{"Functions"}</h3>
                            <p>{"To search based on things cards can be used for, use "}<span class="code">{"fn:"}</span>{". The spefifics of functions will be documented later, but right now you can, for example, search for "}<span class="code">{"fn:\"search deck\""}</span>{"."}</p>
                        </section>
                    </section>
                </div>
            }
        }
    }
}

fn get_filegarden_link(name: &str) -> String {
    format!(
        "https://file.garden/ZJSEzoaUL3bz8vYK/bloodlesscards/{}.png",
        name.replace(' ', "").replace("ä", "a")
    )
}

#[cfg(target_arch = "wasm32")]

pub fn run_future<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    use wasm_bindgen_futures::spawn_local;
    spawn_local(future);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_future<F>(_future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    use futures::executor::LocalPool;
    use futures::task::LocalSpawnExt;

    pub fn run_future<F>(future: F)
    where
        F: std::future::Future<Output = ()> + 'static,
    {
        let mut pool = LocalPool::new();
        pool.spawner().spawn_local(future).unwrap();
        pool.run_until_stalled();
    }
}
