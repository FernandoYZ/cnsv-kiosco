use maud::{DOCTYPE, Markup, html};

/// Base HTML document. `flash` is shown as a toast that fades out.
pub fn page(title: &str, flash: Option<&str>, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="es-PE" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover";
                meta name="theme-color" content="#007aff";
                title { (title) " · Kiosco del Valle" }
                link rel="stylesheet" href="/public/css/app.css";
                script src="/public/js/app.js" defer {}
            }
            body class="min-h-screen bg-ios-bg text-black antialiased" {
                @if let Some(mensaje) = flash {
                    div data-toast role="status"
                        class="fixed top-6 left-1/2 z-[100] -translate-x-1/2 rounded-2xl bg-gray-900 px-5 py-3 text-[15px] font-semibold text-white shadow-xl transition-opacity duration-300" {
                        (mensaje)
                    }
                }
                (content)
            }
        }
    }
}
