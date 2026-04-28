/// <reference types="@sveltejs/kit" />
/// <reference no-default-lib="true" />
/// <reference lib="esnext" />
/// <reference lib="webworker" />

import { build, files, version } from '$service-worker';

declare const self: ServiceWorkerGlobalScope;

const CACHE = `course-audit-${version}`;

// Build assets are versioned by SvelteKit, so we can pre-cache them aggressively.
const CORE_ASSETS = [...build, ...files];

self.addEventListener('install', (event) => {
	event.waitUntil(
		(async () => {
			const cache = await caches.open(CACHE);
			await cache.addAll(CORE_ASSETS);
			await self.skipWaiting();
		})()
	);
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		(async () => {
			const keys = await caches.keys();
			await Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)));
			await self.clients.claim();
		})()
	);
});

self.addEventListener('fetch', (event) => {
	const req = event.request;
	if (req.method !== 'GET') return;

	const url = new URL(req.url);
	if (url.origin !== self.location.origin) return;

	event.respondWith(
		(async () => {
			try {
				const response = await fetch(req);
				const cache = await caches.open(CACHE);
				cache.put(req, response.clone()).catch(() => {
					/* quota or other cache error — ignore silently */
				});
				return response;
			} catch {
				const cached = await caches.match(req);
				if (cached) return cached;
				const fallback = await caches.match('/');
				if (fallback) return fallback;
				throw new Error('Offline and no cached response available');
			}
		})()
	);
});
