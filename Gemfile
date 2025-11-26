# frozen_string_literal: true

source 'https://rubygems.org'

# Bundle edge Rails instead: gem "rails", github: "rails/rails", branch: "main"
gem 'rails', '>= 8.0'
# The modern asset pipeline for Rails [https://github.com/rails/propshaft]
gem 'propshaft'
# Use PostgreSQL as the database for Active Record
gem 'pg', '>= 1.1'
# Use the Puma web server [https://github.com/puma/puma]
gem 'puma', '>= 5.0'
# Use JavaScript with ESM import maps [https://github.com/rails/importmap-rails]
gem 'importmap-rails'
# Hotwire's SPA-like page accelerator [https://turbo.hotwired.dev]
gem 'turbo-rails'
# Hotwire's modest JavaScript framework [https://stimulus.hotwired.dev]
gem 'stimulus-rails'
# Build JSON APIs with ease [https://github.com/rails/jbuilder]
gem 'jbuilder'
# Use Active Model has_secure_password [https://guides.rubyonrails.org/active_model_basics.html#securepassword]
gem 'bcrypt', '>= 3.1'
# Authentication framework [https://rodauth.jeremyevans.net/]
gem 'rodauth-rails', '~> 1.15'
# OAuth integration for Rodauth [https://github.com/janko/rodauth-omniauth]
gem 'rodauth-omniauth', '~> 0.4'
# OAuth provider for Google
gem 'omniauth-google-oauth2', '~> 1.2'
# Authorization framework [https://github.com/varvet/pundit]
gem 'pundit', '>= 2.4'
# Audit logging [https://github.com/paper-trail-gem/paper_trail]
gem 'paper_trail'
# Pagination [https://github.com/ddnexus/pagy]
gem 'pagy', '>= 43.1'

gem 'ostruct'
# Windows does not include zoneinfo files, so bundle the tzinfo-data gem
gem 'tzinfo-data', platforms: %i[windows jruby]
# Use the database-backed adapters for Rails.cache, Active Job, and Action Cable
gem 'solid_cable'
# Solid Cache is a database-backed Active Support cache store [https://github.com/rails/solid_cache]
gem 'solid_cache'
# Solid Queue is a database-based queuing backend for Active Job [https://github.com/rails/solid_queue]
gem 'solid_queue'
# Reduces boot times through caching; required in config/boot.rb
gem 'bootsnap', require: false
# Add HTTP asset caching/compression and X-Sendfile acceleration to Puma [https://github.com/basecamp/thruster/]
gem 'thruster', require: false
# Use Phlex for views [https://github.com/phlex-rb/phlex-rails]
gem 'phlex-rails', '>= 2.3'
gem 'tailwindcss-rails', '>= 4.3'
gem 'tailwind_merge', '>= 1.3'

group :development, :test do
  # See https://guides.rubyonrails.org/debugging_rails_applications.html#debugging-with-the-debug-gem
  gem 'debug', platforms: %i[mri windows], require: 'debug/prelude'

  # Static analysis for security vulnerabilities [https://brakemanscanner.org/]
  gem 'brakeman', require: false

  # Omakase Ruby styling [https://github.com/rails/rubocop-rails-omakase/]
  gem 'rubocop-rails-omakase', require: false

  # Factory Bot [https://github.com/thoughtbot/factory_bot_rails]
  gem 'rubocop-factory_bot', require: false

  # Clean database between tests
  gem 'database_cleaner-active_record'

  # Playwright for end-to-end testing
  gem 'capybara'
  gem 'capybara-playwright-driver'
  gem 'factory_bot_rails'
  gem 'pundit-matchers', '>= 2.0'
  gem 'rails-controller-testing'
  gem 'rspec-github', require: false
  gem 'rspec-rails', '>= 8.0'
  gem 'shoulda-matchers', '>= 6.0'
end

group :development do
  gem 'rubocop-capybara'
  gem 'rubocop-rails'
  gem 'rubocop-rspec'
  gem 'rubocop-rspec_rails'
  gem 'ruby_ui', require: false
  gem 'web-console'
end
