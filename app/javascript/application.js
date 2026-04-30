// Configure your import map in config/importmap.rb. Read more: https://github.com/rails/importmap-rails
import { Turbo } from "@hotwired/turbo-rails"
import "session_expiry"

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
				}
			});
		})
		.catch(function (error) {
			console.log('Service Worker registration failed:', error);
		});
}

function offlineShellPrefetchEligible() {
	return ['/', '/dashboard', '/people', '/medications', '/schedules', '/locations'].some(function (path) {
		return window.location.pathname === path || window.location.pathname.startsWith(path + '/');
	});
}
