mod app;
use app::App;
use yew::prelude::*;

pub fn main() {
    let x = yew::Renderer::<App>::new();
    x.hydrate();
}
