# Kiosco del Valle — Control de consumos escolares

Aplicación web para registrar lo que consumen los estudiantes en el kiosco del colegio, cobrar los saldos y cerrar la caja del día. Es la reescritura en **Rust** de la versión original en Go: trabaja sobre **la misma base de datos SQLite** y se distribuye como un solo binario con todos los estáticos embebidos.

## Inicio rápido

1. Instala [Rust con rustup](https://rustup.rs). La versión exacta (1.100) se instala sola gracias a `rust-toolchain.toml`.
2. Descarga el [CLI standalone de Tailwind CSS v4](https://github.com/tailwindlabs/tailwindcss/releases) en la raíz del proyecto con el nombre `tailwindcss` (está en `.gitignore`).
3. Copia una base de datos existente a `database/database.db`. La aplicación **nunca crea una base vacía**: si no la encuentra, no arranca.
4. Compila el CSS y levanta el servidor:

   ```bash
   ./tailwindcss -i styles/app.css -o public/css/app.css --minify
   cargo run
   ```

5. Abre <http://127.0.0.1:3200> e inicia sesión con un usuario de la tabla `usuarios`.

> [!IMPORTANT]
> Vuelve a compilar el CSS cada vez que cambies clases de Tailwind: los archivos de `public/` se embeben en el binario al compilar.

## Qué hace

| Pantalla | Ruta | Para qué sirve |
|----------|------|----------------|
| Inicio | `/` | Tabla semanal de consumos por grado, con deuda anterior y pagos |
| Registro | `/registro` | Elegir sector (menor: 5to Primaria–2do Secundaria, mayor: 3ro–5to Secundaria) y registrar consumos del día |
| Editor de consumos | `/consumos/{id}` | Sumar o restar productos de un estudiante en un día |
| Resumen del día | `/resumen/{sector}` | Qué consumió cada estudiante y cuántas unidades de cada producto |
| Pagos | `/pagos/{id}` | Registrar o eliminar abonos de la semana (sin pagos duplicados por doble envío) |
| Comprobante | `/comprobante/{id}` | Nota semanal como imagen para copiar o compartir con la familia |
| Cobranza | `/cobranza` | Estudiantes con deuda, de mayor a menor, con mensaje listo para compartir |
| Cierre del día | `/cierre` | Vendido, cobrado y ventas por sector y producto |
| Estudiantes / Productos | `/estudiantes`, `/productos` | Altas, edición y activación o baja |

También es una **PWA instalable** (Chrome y Samsung Internet en Android) y hace **una copia diaria de la base de datos** en `database/backups/`, conservando las últimas 30.

## Usuarios y permisos

Los usuarios viven en la tabla `usuarios` (contraseñas en Argon2id). Solo hay un permiso, `puede_editar`:

| `puede_editar` | Puede usar |
|----------------|-----------|
| `1` | Todo |
| `0` | Registro, resumen del día, editor de consumos y ver productos. Cualquier otra pantalla lo devuelve a `/registro` |

La aplicación no crea usuarios: se administran directamente en la base de datos.

## Configuración

Los valores por defecto están en `config/default.toml`. Cada uno se puede sobrescribir con una variable de entorno:

| Variable | Por defecto | Uso |
|----------|-------------|-----|
| `KIOSCO_CONFIG` | `config/default.toml` | Archivo de configuración |
| `KIOSCO_SERVER_ADDRESS` | `127.0.0.1:3200` | Dirección y puerto del servidor |
| `KIOSCO_DATABASE_PATH` | `database/database.db` | Base de datos SQLite (debe existir) |
| `KIOSCO_BACKUP_DIR` | `database/backups` | Carpeta de las copias diarias |
| `KIOSCO_COOKIE_SECURE` | `false` | `true` en producción (HTTPS) para que las cookies lleven `Secure` |
| `RUST_LOG` | `info,tower_http=info` | Nivel de los logs |

> [!WARNING]
> En producción la aplicación debe escuchar en `127.0.0.1` detrás de un proxy (Caddy). El límite de intentos de login usa la IP que agrega el proxy en `X-Forwarded-For`; expuesta directamente, ese límite se puede evadir.

## Desarrollo

```bash
cargo test                    # Tests (base SQLite en memoria con el esquema real)
cargo clippy --all-targets    # Lint
cargo fmt                     # Formato
cargo build --release         # Binario de producción: target/release/cnsv-kiosco
```

### Base de datos

- El esquema es el de la versión en Go, sin cambios de formato.
- `migrations/001_consumos_unique.sql` une consumos duplicados y evita que vuelvan a aparecer. Aplícala con la aplicación detenida: `sqlite3 database/database.db < migrations/001_consumos_unique.sql`. Si falta, la aplicación avisa en el log al arrancar.
- SQLite trabaja en modo WAL; los guardados de consumos usan transacciones `IMMEDIATE`.

## Seguridad

| Tema | Cómo se resuelve |
|------|------------------|
| Contraseñas | Argon2id, verificadas con los parámetros de cada hash (compatibles con los de Go) |
| Sesión | Cookie `kiosco_session` firmada con HMAC-SHA256, válida 24 h, `HttpOnly` y `SameSite=Lax`. La llave cambia en cada reinicio, así que reiniciar cierra todas las sesiones |
| CSRF | Cookie `kiosco_csrf` comparada con la cabecera `X-CSRF-Token` (la envía `app.js`) o el campo `csrf_token` en cada POST |
| Intentos de login | 5 fallos en 15 minutos por IP; un acierto reinicia el contador |
| Caché | Las páginas se sirven con `no-store`; los estáticos, versionados e inmutables |

## Tecnologías

| Capa | Tecnología |
|------|------------|
| Servidor | Rust 2024, [Axum](https://github.com/tokio-rs/axum) 0.8, Tokio |
| Vistas | [Maud](https://maud.lambda.xyz) (HTML compilado en Rust) |
| Base de datos | SQLite con [tokio-rusqlite](https://github.com/programatik29/tokio-rusqlite) (incluida en el binario) |
| Estilos | Tailwind CSS v4 (CLI standalone, sin Node.js) |
| JavaScript | Vanilla JS propio + [html-to-image](https://github.com/bubkoo/html-to-image) para los comprobantes |
| Seguridad | `argon2`, `hmac` + `sha2`, `subtle` |

## Estructura

```
cnsv-kiosco/
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

## Contacto

**Desarrollador:** Fernando YZ
**GitHub:** [@FernandoYZ](https://github.com/FernandoYZ)
