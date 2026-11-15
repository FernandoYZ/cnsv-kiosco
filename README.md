# Kiosco del Valle

[![CI](https://github.com/FernandoYZ/cnsv-kiosco/actions/workflows/ci.yml/badge.svg)](https://github.com/FernandoYZ/cnsv-kiosco/actions/workflows/ci.yml)
![Rust 1.100](https://img.shields.io/badge/rust-1.100-orange?logo=rust)
![Axum 0.8](https://img.shields.io/badge/axum-0.8-blue)
![SQLite](https://img.shields.io/badge/sqlite-WAL-003B57?logo=sqlite)

Sistema web para el kiosco escolar: registra lo que consume cada estudiante, cobra los saldos semanales, genera comprobantes para las familias y cierra la caja del día.

Es la reescritura en **Rust** de la versión original en Go. Trabaja sobre **la misma base de datos SQLite** y se distribuye como **un solo binario** con todos los estáticos embebidos: en producción usa menos de 10 MB de RAM.

## Contenido

- [Funcionalidades](#funcionalidades)
- [Inicio rápido](#inicio-rápido)
- [Configuración](#configuración)
- [Usuarios y permisos](#usuarios-y-permisos)
- [Desarrollo](#desarrollo)
- [Integración continua y despliegue](#integración-continua-y-despliegue)
- [Base de datos](#base-de-datos)
- [Seguridad](#seguridad)
- [Arquitectura](#arquitectura)

## Funcionalidades

| Pantalla | Ruta | Para qué sirve |
|----------|------|----------------|
| Inicio | `/` | Tabla semanal de consumos por grado, con deuda anterior y pagos |
| Registro | `/registro` | Elegir sector (menor: 5.º de primaria a 2.º de secundaria; mayor: 3.º a 5.º de secundaria) y registrar los consumos del día |
| Editor de consumos | `/consumos/{id}` | Sumar o restar productos de un estudiante en un día |
| Resumen del día | `/resumen/{sector}` | Qué consumió cada estudiante y cuántas unidades se vendieron de cada producto |
| Pagos | `/pagos/{id}` | Registrar o eliminar abonos de la semana, sin duplicados por doble envío |
| Comprobante | `/comprobante/{id}` | Nota semanal como imagen, lista para copiar o compartir con la familia |
| Cobranza | `/cobranza` | Estudiantes con deuda, de mayor a menor, con un mensaje listo para compartir |
| Cierre del día | `/cierre` | Vendido, cobrado y ventas por sector y producto |
| Estudiantes / Productos | `/estudiantes`, `/productos` | Altas, edición, activación y baja |

Además:

- **PWA instalable** en Android (Chrome y Samsung Internet).
- **Copia diaria automática** de la base de datos en `database/backups/`; se conservan las últimas 30.

## Inicio rápido

**Requisitos:** [rustup](https://rustup.rs) y el [CLI standalone de Tailwind CSS v4](https://github.com/tailwindlabs/tailwindcss/releases). No se necesita Node.js.

1. Descarga el CLI de Tailwind en la raíz del proyecto con el nombre `tailwindcss` (está en `.gitignore`). La versión de Rust (1.100) se instala sola gracias a `rust-toolchain.toml`.
2. Copia una base de datos existente a `database/database.db`. La aplicación **nunca crea una base vacía**: si no la encuentra, no arranca.
3. Compila el CSS y levanta el servidor:

   ```bash
   ./tailwindcss -i styles/app.css -o public/css/app.css --minify
   cargo run
   ```

4. Abre <http://127.0.0.1:3200> e inicia sesión con un usuario de la tabla `usuarios`.

> [!IMPORTANT]
> Vuelve a compilar el CSS cada vez que cambies clases de Tailwind: los archivos de `public/` se embeben en el binario al compilar. El CI rechaza los cambios si `public/css/app.css` no está actualizado.

## Configuración

Todos los valores tienen un valor por defecto dentro del binario: para arrancar basta con el binario y `database/database.db`.

**Orden de prioridad:** variables de entorno › `config/default.toml` (si existe) › valores por defecto del binario. Si `KIOSCO_CONFIG` apunta a un archivo que no existe, la aplicación no arranca, porque se asume un error de ruta.

| Variable | Por defecto | Uso |
|----------|-------------|-----|
| `KIOSCO_CONFIG` | `config/default.toml` | Archivo de configuración |
| `KIOSCO_SERVER_ADDRESS` | `127.0.0.1:3200` | Dirección y puerto del servidor |
| `KIOSCO_DATABASE_PATH` | `database/database.db` | Base de datos SQLite (debe existir) |
| `KIOSCO_BACKUP_DIR` | `database/backups` | Carpeta de las copias diarias |
| `KIOSCO_COOKIE_SECURE` | `false` | `true` en producción (HTTPS) para que las cookies lleven `Secure` |
| `RUST_LOG` | `info,tower_http=info` | Nivel de los logs |

## Usuarios y permisos

Los usuarios viven en la tabla `usuarios`, con contraseñas en Argon2id. La aplicación no crea usuarios: se administran directamente en la base de datos.

Hay un solo permiso, `puede_editar`:

| `puede_editar` | Acceso |
|----------------|--------|
| `1` | Todas las pantallas |
| `0` | Registro, resumen del día, editor de consumos y consulta de productos. Cualquier otra pantalla redirige a `/registro` |

## Desarrollo

```bash
cargo test                                  # Tests (SQLite en memoria con el esquema real)
cargo clippy --all-targets -- -D warnings   # Lint, igual que en el CI
cargo fmt                                   # Formato
cargo build --release                       # Binario de producción: target/release/cnsv-kiosco
```

Los tests cubren las reglas de negocio (pagos sin duplicados, consumos transaccionales, cierre y cobranza) y la seguridad (login, sesión, CSRF, permisos y límite de intentos) llamando al router completo contra una base SQLite real.

## Integración continua y despliegue

| Workflow | Se ejecuta con | Qué hace |
|----------|----------------|----------|
| [`ci.yml`](.github/workflows/ci.yml) | Push a `main` y pull requests | Verifica que el CSS compilado esté al día, `cargo fmt --check`, `clippy -D warnings` y `cargo test` |
| [`release.yml`](.github/workflows/release.yml) | Push de un tag `vX.Y.Z` que esté en `main` | Corre el CI, compila en Ubuntu 22.04 (compatible con Debian 12), despliega en el servidor con reversión automática si `/health` falla y publica el binario en GitHub Releases |

**Publicar una versión:**

1. Actualiza `version` en `Cargo.toml` y haz commit en `main` (`chore: release vX.Y.Z`).
2. Espera a que el CI pase en verde.
3. Crea y sube el tag:

   ```bash
   git tag vX.Y.Z && git push origin vX.Y.Z
   ```

El despliegue reinicia el servicio, así que se cierran las sesiones abiertas: conviene publicar fuera del horario de atención. Usa los secretos `DEPLOY_HOST`, `DEPLOY_USER`, `DEPLOY_PORT`, `DEPLOY_SSH_KEY` y `DEPLOY_KNOWN_HOSTS` del entorno `production` de GitHub.

## Base de datos

- El esquema es el de la versión en Go, sin cambios de formato.
- SQLite trabaja en modo WAL; los guardados de consumos usan transacciones `IMMEDIATE`.
- Las migraciones son scripts SQL manuales en `migrations/`. Se aplican **con la aplicación detenida** y después de un backup:

  ```bash
  sqlite3 database/database.db < migrations/001_consumos_unique.sql
  ```

  Si la migración 001 falta, la aplicación lo avisa en el log al arrancar.

## Seguridad

| Tema | Cómo se resuelve |
|------|------------------|
| Contraseñas | Argon2id, verificadas con los parámetros de cada hash (compatibles con los de Go) |
| Sesión | Cookie `kiosco_session` firmada con HMAC-SHA256, válida 24 h, `HttpOnly` y `SameSite=Lax`. La llave cambia en cada reinicio, así que reiniciar cierra todas las sesiones |
| CSRF | Cookie `kiosco_csrf` comparada con la cabecera `X-CSRF-Token` (la envía `app.js`) o con el campo `csrf_token` en cada POST |
| Cierre de sesión | Solo `POST /logout` con token CSRF: un enlace o una imagen en otro sitio no puede cerrar la sesión. `GET /logout` responde `405` |
| Intentos de login | 5 fallos en 15 minutos por IP; un acierto reinicia el contador |
| Caché | Las páginas se sirven con `no-store`; los estáticos, versionados e inmutables |

> [!WARNING]
> En producción la aplicación debe escuchar en `127.0.0.1` detrás de un proxy inverso. El límite de intentos de login toma la **última** IP de `X-Forwarded-For`, así que el proxy debe enviar ahí la IP real del cliente. Si hay una CDN delante (por ejemplo, Cloudflare), el proxy debe confiar solo en sus rangos y tomar la IP de su cabecera; de lo contrario, todos los usuarios comparten el mismo contador.

## Arquitectura

Monolito modular organizado por funcionalidad: cada módulo separa el acceso HTTP (`handler`) de las consultas SQL (`repository`), y agrega una capa de servicio solo donde hay lógica transaccional.

```text
cnsv-kiosco/
├── .github/workflows/        # CI y despliegue por tags
├── src/
│   ├── main.rs, app.rs       # Arranque y rutas
│   ├── auth/                 # Login, sesión, CSRF, límite de intentos, permisos
│   ├── dashboard/            # Tabla semanal (Inicio)
│   ├── consumos/             # Registro, editor y resumen del día
│   ├── pagos/                # Pagos y comprobante
│   ├── cobranza/, cierre/    # Deudores y cierre del día
│   ├── estudiantes/, productos/
│   ├── templates/            # Layout y componentes Maud
│   ├── backup.rs, db.rs, config.rs, assets.rs
│   └── test_support.rs       # Esquema y datos de prueba
├── public/                   # Estáticos embebidos (CSS compilado, JS, íconos, manifest, sw.js)
├── styles/app.css            # Entrada de Tailwind
├── migrations/               # Scripts SQL manuales
├── config/default.toml
└── database/                 # database.db y backups/ (fuera de git)
```

| Capa | Tecnología |
|------|------------|
| Servidor | Rust 2024, [Axum](https://github.com/tokio-rs/axum) 0.8, Tokio |
| Vistas | [Maud](https://maud.lambda.xyz) (HTML compilado en Rust) |
| Base de datos | SQLite con [tokio-rusqlite](https://github.com/programatik29/tokio-rusqlite) (incluida en el binario) |
| Estilos | Tailwind CSS v4 (CLI standalone, sin Node.js) |
| JavaScript | Vanilla JS propio + [html-to-image](https://github.com/bubkoo/html-to-image) para los comprobantes |
| Seguridad | `argon2`, `hmac` + `sha2`, `subtle` |

## Autor

**Fernando YZ** · [@FernandoYZ](https://github.com/FernandoYZ)
