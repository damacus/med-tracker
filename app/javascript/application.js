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
					fetch('/offline', { credentials: 'same-origin', headers: { Accept: 'text/html' } }).catch(function () {});
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
	return ['/', '/dashboard', '/people', '/medications', '/schedules', '/locations'].some(function (path) {
		return window.location.pathname === path || window.location.pathname.startsWith(path + '/');
	});
}

async function runOfflineSync() {
	if (!navigator.onLine || !offlineShellPrefetchEligible()) return;

	try {
		await refreshSnapshot('/offline/snapshot');
		const result = await syncQueuedTakes('/offline/medication_takes');
		if (result.synced.length > 0) await refreshSnapshot('/offline/snapshot');
	} catch (_) {}
}
