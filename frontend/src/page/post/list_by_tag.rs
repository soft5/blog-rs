use yew::prelude::*;

use crate::component::PostsListComponent;

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct Props {
    pub tag_name: String,
}

pub struct PostsListByTag {
    tag_name: String,
}

impl Component for PostsListByTag {
    type Message = ();
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            tag_name: String::from(&ctx.props().tag_name),
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        let changed = self.tag_name.ne(&ctx.props().tag_name);
        if changed {
            weblog::console_log!("changed to load");
            self.tag_name.clear();
            self.tag_name.push_str(&ctx.props().tag_name);
        }
        changed
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let Self { tag_name } = self;
        let mut request_uri = String::with_capacity(32);
        request_uri.push_str("/post/tag/");
        request_uri.push_str(tag_name);
        request_uri.push_str("/");

        let decoded_tag_name = urlencoding::decode(tag_name).unwrap();

        gloo::utils::document().set_title(&decoded_tag_name);

        html! {
            <>
                <div class="columns">
                    <div class="column is-10">
                        <h1 class="title is-1">{ decoded_tag_name }</h1>
                        <h2 class="subtitle">{ " " }</h2>
                    </div>
                </div>
                <PostsListComponent {request_uri} />
            </>
        }
    }
}
