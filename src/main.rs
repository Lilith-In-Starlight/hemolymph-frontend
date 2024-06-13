mod app;
use app::App;

pub fn main() {
    let x = yew::Renderer::<App>::new();
    x.hydrate();
}
