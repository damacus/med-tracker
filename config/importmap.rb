# Pin npm packages by running ./bin/importmap

pin "application"
pin "@hotwired/turbo-rails", to: "turbo.min.js", preload: true
pin "@hotwired/stimulus", to: "stimulus.min.js", preload: true
pin "@hotwired/stimulus-loading", to: "stimulus-loading.js", preload: true

# Pin controllers
pin_all_from "app/javascript/controllers", under: "controllers"
pin "controllers", to: "controllers/index.js"
pin "controllers/application", to: "controllers/application.js"
