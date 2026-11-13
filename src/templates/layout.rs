use maud::{DOCTYPE, Markup, html};

use crate::assets;

/// Base HTML document. `flash` is shown as a toast that fades out.
pub fn page(title: &str, flash: Option<&str>, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="es-PE" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover";
                // Installable as a home-screen app (standalone, no browser bar), as the Go app.
                meta name="theme-color" content="#01398d";
                meta name="mobile-web-app-capable" content="yes";
                meta name="apple-mobile-web-app-capable" content="yes";
                meta name="apple-mobile-web-app-status-bar-style" content="default";
                meta name="apple-mobile-web-app-title" content="Kiosco";
                title { (title) " · Kiosco del Valle" }
                link rel="icon" href=(assets::url("images/icon-192.png")) type="image/png";
                link rel="apple-touch-icon" href=(assets::url("images/apple-touch-icon.png"));
                link rel="manifest" href=(assets::url("manifest.json"));
                // Same URL as the CSS `url(...)`, so the preload is reused.
                link rel="preload" href="/public/fonts/inter-latin.woff2" as="font" type="font/woff2" crossorigin;
                link rel="stylesheet" href=(assets::url("css/app.css"));
                script src=(assets::url("js/app.js")) defer {}
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
