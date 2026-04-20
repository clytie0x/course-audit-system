// Minimal service worker: network-first for navigation + WASM/JS assets so
// new deploys land immediately, but fall back to cached copies when offline.
// Keeps the privacy-first guarantee — we never cache user data, only the
// static app shell.

const CACHE = 'course-audit-v1';
const CORE_ASSETS = [
    './',
    './index.html',
    './manifest.webmanifest',
];

self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open(CACHE).then((cache) => cache.addAll(CORE_ASSETS)).catch(() => { })
    );
    self.skipWaiting();
});

self.addEventListener('activate', (event) => {
    event.waitUntil(
        caches.keys().then((keys) =>
            Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)))
        )
    );
    self.clients.claim();
});

self.addEventListener('fetch', (event) => {
    const req = event.request;
    if (req.method !== 'GET') return;
    const url = new URL(req.url);

    // Only handle same-origin assets — PDF.js stays on the CDN, student PDFs
    // are never fetched (they're loaded via File input and stay in memory).
    if (url.origin !== self.location.origin) return;

    event.respondWith(
        fetch(req)
            .then((response) => {
                const copy = response.clone();
                caches.open(CACHE).then((cache) => cache.put(req, copy)).catch(() => { });
                return response;
            })
            .catch(() => caches.match(req).then((cached) => cached || caches.match('./index.html')))
    );
});
