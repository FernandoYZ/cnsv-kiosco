# Changelog

Cambios de cada versión, pensados para quien usa el sistema. Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/); las versiones siguen [Semantic Versioning](https://semver.org/lang/es/).

La sección de cada versión se publica tal cual como notas de la Release en GitHub: sin ella, la versión no se despliega.

## [Sin publicar]

### Corregido

- **Semanas que cruzan de mes.** Una semana como la del 30 de noviembre al 5 de diciembre aparecía como «30-5/Diciembre/2026». Ahora muestra los dos meses, tanto en Pagos («Semana del 30 de Noviembre al 5 de Diciembre») como en el comprobante («30/Noviembre-5/Diciembre/2026»), y también los dos años cuando la semana cruza de año.
- **Saber qué día se está tocando en el celular.** En la tabla semanal, al bajar por la lista los días de las columnas quedaban fuera de la pantalla. Ahora cada celda muestra su día (por ejemplo «Mar 10/11») en el celular; en la computadora se sigue viendo solo el encabezado.
- **Nombres de productos partidos.** En la tabla semanal, nombres como «Salchipapa» o «Chicharrón» se cortaban a mitad de palabra («Salchipap / a»). Ahora cada celda muestra los productos en dos columnas y los nombres se leen enteros.
- **Pagar de más ya no deja montos negativos.** Si un estudiante abonaba más de lo que debía, el comprobante decía «Total a pagar: S/ -2.00» y la semana siguiente mostraba una deuda anterior negativa. Ahora el comprobante dice «Saldo a favor: S/ 2.00», la semana siguiente lo descuenta como «Saldo a favor anterior», y la tabla semanal y la página de pagos lo muestran como «S/ 2.00 a favor», en verde.

### Cambiado

- **Columnas de cuenta sin nada que cobrar, en gris azulado.** En la tabla semanal, Subtotal y Deuda anterior solo se resaltan en naranja cuando hay algo que cobrar, y Pagos solo en amarillo cuando hay deuda; si no, se ven en un gris azulado que se distingue de los días.

## [1.0.2] - 2026-11-15

### Corregido

- **Guardar consumos sin pasar por la lista del sector.** Si el editor de consumos se abría desde un enlace directo, al guardar el cajero era enviado a una pantalla que no puede ver y terminaba en el Registro sin el aviso «Consumo guardado». Ahora vuelve a la lista del sector del estudiante, con el aviso, y el botón para volver también lleva ahí.
- **Productos para usuarios sin permiso de edición.** Quien no puede editar veía el formulario para agregar productos y los botones «Editar» y «Desactivar», pero al usarlos no pasaba nada. Ahora solo ve el catálogo, a todo el ancho.

## [1.0.1] - 2026-11-15

### Corregido

- **Cierre de sesión desde otro sitio (seguridad).** Antes, un enlace o una imagen en cualquier página podía cerrar tu sesión sin que lo pidieras. Ahora la sesión solo se cierra con el botón de la app.
- **Cerrar sesión sacaba de la app.** Si cerrabas sesión justo después de entrar, el navegador volvía a la página anterior (por ejemplo, una pestaña en blanco) en lugar de mostrar el inicio de sesión.
- **Montos de S/ 0.00.** Un monto como `0.001` en un pago o en el precio de un producto se guardaba como S/ 0.00. Ahora se rechaza con el mensaje de monto inválido.

### Cambiado

- «Cerrar sesión» y «Salir» ahora son botones; se ven igual que antes.

## [1.0.0] - 2026-11-14

Primera versión en Rust: reemplaza a la aplicación en Go sobre la misma base de datos.

### Agregado

- Registro de consumos por sector y editor por estudiante, con resumen del día.
- Tabla semanal por grado con deuda anterior y pagos.
- Pagos sin duplicados por doble envío y comprobante semanal como imagen para compartir.
- Cobranza (estudiantes con deuda) y cierre del día.
- Altas, edición y baja de estudiantes y productos.
- Inicio de sesión con dos niveles de permiso y límite de intentos.
- Aplicación instalable en Android (PWA) y copia diaria automática de la base de datos.

[1.0.2]: https://github.com/FernandoYZ/cnsv-kiosco/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/FernandoYZ/cnsv-kiosco/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/FernandoYZ/cnsv-kiosco/releases/tag/v1.0.0
