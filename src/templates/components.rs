use maud::{Markup, html};

pub fn page_header(title: &str) -> Markup {
    html! {
        header class="mb-6" {
            h1 class="text-2xl font-semibold tracking-tight" { (title) }
        }
    }
}
