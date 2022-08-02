use blog_frontend::app::App;
use yew::{html, prelude::*};
use yew_router::prelude::*;

#[function_component(Main)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <App />
        </BrowserRouter>
    }
}

fn main() {
    yew::start_app::<Main>();
    // yew::Renderer::<Main>::new().render();
}
