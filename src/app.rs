#![warn(clippy::pedantic)]
mod card_details;
use card_details::CardDetails;
use std::collections::HashMap;
use std::sync::Mutex;

use gloo_timers::callback::Timeout;
use hemoglobin::cards::Card;
use rand::seq::SliceRandom;
use reqwest::Client;
use serde::Deserialize;
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
pub static HOST: &str = "127.0.0.1";
#[cfg(debug_assertions)]
pub static PORT: &str = "8080";

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
struct CardListProps {
    search: String,
}

#[derive(Deserialize, PartialEq)]
#[serde(tag = "type")]
enum QueryResult {
    CardList {
        query_text: String,
        content: Vec<Card>,
    },
    Error {
        message: String,
    },
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
        Some(QueryResult::CardList {
            query_text,
            content,
        }) => {
            let a = content
                .iter()
                .map(|card| {
                    html! {
                        <Link<Route> to={Route::Card{id: card.id.clone()}}><img class="card-result" src={get_filegarden_link(card.img.choose(&mut rand::thread_rng()).unwrap_or(&card.name))} /></Link<Route>>
                    }
                });

            html! {
                <>
                    <p id="query_readable">{"Showing "}{a.len()}{" "}{query_text}</p>
                    <div id="results">
                        {for a}
                    </div>
                </>
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
    let fallback = html! {<div><p>{"Loading..."}</p></div>};
    match route {
        Route::Search { query } => {
            html! {<Suspense fallback={fallback}><CardList search={query} /></Suspense>}
        }
        Route::Card { id } => {
            html! {<Suspense fallback={fallback}> <CardDetails card_id={id}/> </Suspense>}
        }
        Route::Instructions => {
            modify_title("How To");
            html! {
                <section id="instructions">
                    <h2>{"How to use Hemolymph"}</h2>
                    <p>{"Hemolymph is the arthropod equivalent of blood. It is also Bloodless' official card database."}</p>
                    <div id="instructions-grid">
                        <section class="instruction fuzzy_instr">
                            <h3>{"Fuzzy Search"}</h3>
                            <p>{"By default, your searches look for matches in Name, Kins, Keywords and Description, prioritizing them in that order."}</p>
                        </section>
                        <section class="instruction name_instr">
                            <h3>{"Name"}</h3>
                            <p>{"If you want to search by name only, you can write "}<span class="code">{"name:"}</span>{" or "}<span class="code">{"n:"}</span>{" before the name."}</p>
                            <p class="code">{"n:mantis"}</p>
                            <p>{"Surround the name in quotation marks if it contains spaces."}</p>
                            <p class="code">{"n:\"lost man\""}</p>
                        </section>
                        <section class="instruction kins_instr">
                            <h3>{"Kins, Types and Keywords"}</h3>
                            <p>{"You can use "}<span class="code">{"k:"}</span>{" for kins and "}<span class="code">{"kw:"}</span>{" for keywords. If you want to match more than one kin, they have to be separate. To search by type, use "} <span class="code">{"t:"}</span>{"."}</p>
                            <p class="code">{"k:ant kw:\"flying defense\" t:creature"}</p>
                        </section>
                        <section class="instruction stats_instr">
                            <h3>{"Stats"}</h3>
                            <p>{"You can use "}<span class="code">{"h: d: p:"}</span>{" and "}<span class="code">{"c:"}</span>{" for health, defense, power and strength, respectively. You can also match comparisons."}</p>
                            <p class="code">{"c=2 p>1 d<2 h!=1"}</p>
                        </section>
                        <section class="instruction devours">
                            <h3>{"Devours"}</h3>
                            <p>{"To look for cards that devour other cards, you use "}<span class="code">{"devours:"}</span>{" or "}<span class="code">{"dev:"}</span>{", which require a search query inside them, wrapped in parentheses."}</p>
                            <p class="code">{"devours:(cost=1)"}</p>
                        </section>
                        <section class="instruction fn_instr">
                            <h3>{"Functions"}</h3>
                            <p>{"To search based on things cards can be used for, use "}<span class="code">{"fn:"}</span>{". The spefifics of functions will be documented later, but right now you can, for example, search for "}<span class="code">{"fn:\"search deck\""}</span>{"."}</p>
                        </section>
                        <section class="instruction negation">
                            <h3>{"Negation"}</h3>
                            <p>{"You can invert a query's result by putting a dash before it. The following example matches all cards without \"mantis\" in their name."}</p>
                            <p class="code">{"-n:mantis"}</p>
                        </section>
                    </div>
                </section>
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
