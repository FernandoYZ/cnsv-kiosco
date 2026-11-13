use maud::{DOCTYPE, Markup, html};

/// Base HTML document shared by every page.
pub fn page(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="es" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · Kiosco CNSV" }
                link rel="stylesheet" href="/public/css/app.css";
                script src="/public/js/app.js" defer {}
            }
            body class="min-h-screen bg-slate-50 text-slate-900 antialiased" {
                main class="mx-auto max-w-5xl p-6" { (content) }
            }
        }
    }
}
