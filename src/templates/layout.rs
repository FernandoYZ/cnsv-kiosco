use maud::{DOCTYPE, Markup, html};

/// (href, label, key) of the main navigation.
const NAV: [(&str, &str, &str); 5] = [
    ("/", "Inicio", "inicio"),
    ("/registro", "Registro diario", "registro"),
    ("/resumen", "Resumen del día", "resumen"),
    ("/estudiantes", "Estudiantes", "estudiantes"),
    ("/productos", "Productos", "productos"),
];

/// Base HTML document shared by every page. `active` is a key from `NAV`.
pub fn page(title: &str, active: &str, content: Markup) -> Markup {
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
                header class="border-b border-slate-200 bg-white print:hidden" {
                    nav class="mx-auto flex max-w-7xl flex-wrap items-center gap-1 px-4 py-2" {
                        a href="/" class="mr-4 font-semibold text-emerald-700" { "Kiosco CNSV" }
                        @for (href, label, key) in NAV {
                            a href=(href) class=(nav_class(key == active)) { (label) }
                        }
                    }
                }
                main class="mx-auto max-w-7xl p-4 sm:p-6" { (content) }
            }
        }
    }
}

fn nav_class(active: bool) -> &'static str {
    if active {
        "rounded-md bg-emerald-50 px-3 py-1.5 text-sm font-medium text-emerald-800"
    } else {
        "rounded-md px-3 py-1.5 text-sm text-slate-600 hover:bg-slate-100"
    }
}
