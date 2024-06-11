mod search;
use std::sync::Mutex;

use rand::seq::SliceRandom;
use reqwest::Client;
use rust_fuzzy_search::fuzzy_compare;
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Card {
    id: String,
    cost: usize,
    #[serde(default)]
    img: Vec<String>,
    name: String,
    health: usize,
    defense: usize,
    #[serde(default)]
    kins: Vec<String>,
    #[serde(default)]
    keywords: Vec<Keyword>,
    r#type: String,
    power: usize,
    description: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct CardID {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub keywords: Option<Vec<Keyword>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub kins: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defense: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub abilities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub functions: Option<Vec<String>>,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum KeywordData {
    CardID(CardID),
    String(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Keyword {
    pub name: String,
    pub data: Option<KeywordData>,
}

pub enum Comparison {
    GreaterThan(usize),
    GreaterThanOrEqual(usize),
    LowerThanOrEqual(usize),
    Equal(usize),
    LowerThan(usize),
    NotEqual(usize),
}

pub enum QueryRestriction {
    Fuzzy(String),
    Comparison(Box<dyn Fn(&Card) -> usize>, Comparison),
    Contains(Box<dyn Fn(&Card) -> &str>, String),
    Has(Box<dyn Fn(&Card) -> &[String]>, String),
    HasKw(Box<dyn Fn(&Card) -> &[Keyword]>, String),
}

pub enum Errors {
    InvalidComparisonString,
    UnknownParam,
}

enum TokenMode {
    Word,
    Param(String),
    QParam(String),
    SParam(String),
}
fn tokenize_query(q: &str) -> Result<Vec<Token>, Errors> {
    let mut tokens = vec![];
    let mut word = String::new();
    let mut mode = TokenMode::Word;
    let mut paren_count = 0;
    for ch in q.chars().chain(vec!['\n']) {
        match mode {
            TokenMode::Word => match ch {
                ' ' | '\n' => {
                    tokens.push(Token::Word(word));
                    word = String::new();
                }
                ':' => {
                    mode = TokenMode::Param(word);
                    word = String::new();
                }
                ch => word.push(ch),
            },
            TokenMode::Param(ref param) => match ch {
                ' ' | '\n' => {
                    tokens.push(Token::Param(param.to_string(), word));
                    word = String::new();
                    mode = TokenMode::Word;
                }
                '"' if word.is_empty() => {
                    mode = TokenMode::QParam(param.to_string());
                }
                '{' if word.is_empty() => {
                    mode = TokenMode::SParam(param.to_string());
                }
                ch => word.push(ch),
            },
            TokenMode::QParam(ref param) => match ch {
                '"' => {
                    tokens.push(Token::Param(param.to_string(), word));
                    word = String::new();
                    mode = TokenMode::Word;
                }
                ch => word.push(ch),
            },
            TokenMode::SParam(ref param) => match ch {
                ')' if paren_count == 0 => {
                    tokens.push(Token::SuperParam(param.to_string(), tokenize_query(&word)?));
                    word = String::new();
                    mode = TokenMode::Word;
                }
                '(' => {
                    paren_count += 1;
                    word.push(ch);
                }
                ')' if paren_count > 0 => {
                    paren_count -= 1;
                    word.push(ch);
                }
                ch => word.push(ch),
            },
        }
    }
    Ok(tokens)
}
fn text_comparison_parser(s: &str) -> Result<Comparison, Errors> {
    match s.parse::<usize>() {
        Ok(x) => Ok(Comparison::Equal(x)),
        Err(_) => {
            if let Some(end) = s.strip_prefix(">=") {
                end.parse::<usize>()
                    .map(Comparison::GreaterThanOrEqual)
                    .map_err(|_| Errors::InvalidComparisonString)
            } else if let Some(end) = s.strip_prefix("<=") {
                end.parse::<usize>()
                    .map(Comparison::LowerThanOrEqual)
                    .map_err(|_| Errors::InvalidComparisonString)
            } else if let Some(end) = s.strip_prefix('>') {
                end.parse::<usize>()
                    .map(Comparison::GreaterThan)
                    .map_err(|_| Errors::InvalidComparisonString)
            } else if let Some(end) = s.strip_prefix('<') {
                end.parse::<usize>()
                    .map(Comparison::LowerThan)
                    .map_err(|_| Errors::InvalidComparisonString)
            } else if let Some(end) = s.strip_prefix('=') {
                end.parse::<usize>()
                    .map(Comparison::Equal)
                    .map_err(|_| Errors::InvalidComparisonString)
            } else if let Some(end) = s.strip_prefix("!=") {
                end.parse::<usize>()
                    .map(Comparison::NotEqual)
                    .map_err(|_| Errors::InvalidComparisonString)
            } else {
                Err(Errors::InvalidComparisonString)
            }
        }
    }
}
pub fn query_parser(q: &str) -> Result<Vec<QueryRestriction>, Errors> {
    let q = tokenize_query(q)?;
    let mut restrictions = vec![];
    let mut string = String::new();
    for word in &q {
        match word {
            Token::Word(x) => {
                string.push_str(x);
                string.push(' ');
            }
            Token::Param(param, value) => match param.as_str() {
                "cost" | "c" => {
                    let cmp = text_comparison_parser(value)?;
                    restrictions.push(QueryRestriction::Comparison(Box::new(Card::get_cost), cmp));
                }
                "health" | "h" | "hp" => {
                    let cmp = text_comparison_parser(value)?;
                    restrictions.push(QueryRestriction::Comparison(
                        Box::new(Card::get_health),
                        cmp,
                    ));
                }
                "power" | "strength" | "damage" | "p" | "dmg" | "str" => {
                    let cmp = text_comparison_parser(value)?;
                    restrictions.push(QueryRestriction::Comparison(Box::new(Card::get_power), cmp));
                }
                "defense" | "def" | "d" => {
                    let cmp = text_comparison_parser(value)?;
                    restrictions.push(QueryRestriction::Comparison(
                        Box::new(Card::get_defense),
                        cmp,
                    ));
                }
                "name" | "n" => restrictions.push(QueryRestriction::Contains(
                    Box::new(Card::get_name),
                    value.clone(),
                )),
                "type" | "t" => restrictions.push(QueryRestriction::Contains(
                    Box::new(Card::get_type),
                    value.clone(),
                )),
                "kin" | "k" => restrictions.push(QueryRestriction::Has(
                    Box::new(Card::get_kins),
                    value.clone(),
                )),
                "keyword" | "kw" => restrictions.push(QueryRestriction::HasKw(
                    Box::new(Card::get_keywords),
                    value.clone(),
                )),
                _ => return Err(Errors::UnknownParam),
            },
            Token::SuperParam(param, value) => {
                if param == "devour" {
                    todo!();
                }
            }
        }
    }
    let string = string.trim().to_string();
    restrictions.push(QueryRestriction::Fuzzy(string));
    Ok(restrictions)
}

enum Token {
    Word(String),
    Param(String, String),
    SuperParam(String, Vec<Token>),
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
            Err(error) => QueryResult::Error {
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

                        <Link<Route> to={Route::Card{id: card.id.clone()}}><img class="card-result" src={get_filegarden_link(&card.img.choose(&mut rand::thread_rng()).unwrap_or(&card.name))} /></Link<Route>>
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

impl Comparison {
    pub fn compare<T: PartialOrd<usize>>(&self, a: &T) -> bool {
        match self {
            Comparison::GreaterThan(x) => a > x,
            Comparison::Equal(x) => a == x,
            Comparison::LowerThan(x) => a < x,
            Comparison::NotEqual(x) => a != x,
            Comparison::GreaterThanOrEqual(x) => a >= x,
            Comparison::LowerThanOrEqual(x) => a <= x,
        }
    }
}

impl Card {
    pub fn get_cost(&self) -> usize {
        self.cost
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_type(&self) -> &str {
        &self.r#type
    }
    pub fn get_kins(&self) -> &[String] {
        &self.kins
    }
    pub fn get_keywords(&self) -> &[Keyword] {
        &self.keywords
    }
    pub fn get_health(&self) -> usize {
        self.health
    }
    pub fn get_power(&self) -> usize {
        self.power
    }
    pub fn get_defense(&self) -> usize {
        self.defense
    }
}
