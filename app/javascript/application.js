// Configure your import map in config/importmap.rb. Read more: https://github.com/rails/importmap-rails
import { Turbo } from "@hotwired/turbo-rails"

// Enable debug mode
Turbo.setProgressBarDelay(0)
Turbo.session.drive = true

// Import all controllers
import "controllers"

if ('serviceWorker' in navigator) {
	// Register the service worker
	navigator.serviceWorker.register('/service-worker.js')
	  .then(function(registration) {
		console.log('Service Worker registered with scope:', registration.scope);
	  })
	  .catch(function(error) {
		console.log('Service Worker registration failed:', error);
	  });
  }
