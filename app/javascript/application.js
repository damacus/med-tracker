// Configure your import map in config/importmap.rb. Read more: https://github.com/rails/importmap-rails
import { Turbo } from "@hotwired/turbo-rails"
import "session_expiry"

// Enable debug mode
// Turbo.config.drive.progressBarDelay = 0
// Turbo.config.drive.enabled = true

// Import all controllers
import "controllers"

if ('serviceWorker' in navigator) {
	// Register the service worker
	navigator.serviceWorker.register('/service-worker.js')
		.then(function (registration) {
			console.log('Service Worker registered with scope:', registration.scope);
		})
		.catch(function (error) {
			console.log('Service Worker registration failed:', error);
		});
}
