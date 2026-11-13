// Service worker. Its job is to make the app installable as a real app
// (Chrome, Samsung Internet): those browsers want a service worker that
// handles `fetch`. Nothing is cached, so balances and consumption are never
// shown stale; without connection, page loads show a short notice instead.

const OFFLINE_HTML = `<!doctype html>
<html lang="es-PE">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Sin conexión · Kiosco del Valle</title>
  <style>
    body { margin: 0; min-height: 100vh; display: grid; place-items: center;
      background: #f2f2f7; font-family: system-ui, sans-serif; color: #111827; text-align: center; }
    main { padding: 24px; max-width: 320px; }
    h1 { font-size: 22px; margin: 0 0 8px; }
    p { color: #6b7280; margin: 0 0 24px; }
    button { border: 0; border-radius: 14px; background: #007aff; color: #fff;
      font-size: 16px; font-weight: 600; padding: 12px 24px; }
  </style>
</head>
<body>
  <main>
    <h1>Sin conexión</h1>
    <p>Revisa el internet del dispositivo e intenta de nuevo.</p>
    <button onclick="location.reload()">Reintentar</button>
  </main>
</body>
</html>`;

self.addEventListener('install', () => self.skipWaiting());

self.addEventListener('activate', (event) => event.waitUntil(self.clients.claim()));

self.addEventListener('fetch', (event) => {
  const { request } = event;
  // Only page loads; assets and form posts go straight to the network.
  if (request.mode !== 'navigate' || request.method !== 'GET') return;
  event.respondWith(
    fetch(request).catch(
      () => new Response(OFFLINE_HTML, {
        status: 503,
        headers: { 'Content-Type': 'text/html; charset=utf-8' },
      }),
    ),
  );
});
