// Configure your import map in config/importmap.rb. Read more: https://github.com/rails/importmap-rails
import { Turbo } from "@hotwired/turbo-rails"

// Enable debug mode
Turbo.setProgressBarDelay(0)
Turbo.session.drive = true

// Import all controllers
import "controllers"
