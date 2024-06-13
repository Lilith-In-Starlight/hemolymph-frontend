mod app;
use app::App;
use yew::prelude::*;

pub fn main() {
    yew::Renderer::<App>::new().render();
}
