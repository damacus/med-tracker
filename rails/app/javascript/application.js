// Configure your import map in config/importmap.rb. Read more: https://github.com/rails/importmap-rails
import { Turbo } from "@hotwired/turbo-rails"
import "session_expiry"
import { refreshSnapshot, syncQueuedTakes } from "controllers/offline_store"

// Import all controllers
import "controllers"

if ('serviceWorker' in navigator) {
	// Register the service worker
	navigator.serviceWorker.register('/service-worker.js')
		.then(function (registration) {
			console.log('Service Worker registered with scope:', registration.scope);
			navigator.serviceWorker.ready.then(function () {
				if (offlineShellPrefetchEligible()) {
					fetch(`${offlineBasePath()}/offline`, { credentials: 'same-origin', headers: { Accept: 'text/html' } }).catch(function () {});
					runOfflineSync();
				}
			});
		})
		.catch(function (error) {
			console.log('Service Worker registration failed:', error);
	});
}

window.addEventListener('online', runOfflineSync)

function offlineShellPrefetchEligible() {
	const basePath = offlineBasePath();
	if (!basePath) return window.location.pathname === '/';

	return ['', 'dashboard', 'people', 'medications', 'schedules', 'locations'].some(function (path) {
		const candidate = path ? `${basePath}/${path}` : basePath;
		return window.location.pathname === candidate || window.location.pathname.startsWith(candidate + '/');
	});
}

function offlineTenantKey() {
	const householdId = metaContent('med-tracker-household-id');
	const membershipId = metaContent('med-tracker-membership-id');

	if (householdId && membershipId) return `household:${householdId}:membership:${membershipId}`;
	if (householdId) return `household:${householdId}`;

	return 'global';
}

function offlineBasePath() {
	const slug = metaContent('med-tracker-household-slug');
	return slug ? `/households/${slug}` : '';
}

function metaContent(name) {
	return document.querySelector(`meta[name='${name}']`)?.content || '';
}

async function runOfflineSync() {
	if (!navigator.onLine || !offlineShellPrefetchEligible()) return;

	try {
		const basePath = offlineBasePath();
		const tenantKey = offlineTenantKey();
		await refreshSnapshot(`${basePath}/offline/snapshot`, tenantKey);
		const result = await syncQueuedTakes(`${basePath}/offline/medication_takes`, tenantKey);
		if (result.synced.length > 0) await refreshSnapshot(`${basePath}/offline/snapshot`, tenantKey);
	} catch (_) {}
}
