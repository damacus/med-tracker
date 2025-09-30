# frozen_string_literal: true

# Pin npm packages by running ./bin/importmap

pin 'application'
pin '@hotwired/turbo-rails', to: 'turbo.min.js', preload: true
pin '@hotwired/stimulus', to: 'stimulus.min.js', preload: true
pin '@hotwired/stimulus-loading', to: 'stimulus-loading.js', preload: true

# Pin controllers
pin_all_from 'app/javascript/controllers', under: 'controllers'
pin 'controllers', to: 'controllers/index.js'
pin 'controllers/application', to: 'controllers/application.js'
pin "@floating-ui/dom", to: "@floating-ui--dom.js" # @1.7.4
pin "@floating-ui/core", to: "@floating-ui--core.js" # @1.7.3
pin "@floating-ui/utils", to: "@floating-ui--utils.js" # @0.2.10
pin "@floating-ui/utils/dom", to: "@floating-ui--utils--dom.js" # @0.2.10
