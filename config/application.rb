# frozen_string_literal: true

require_relative 'boot'

require 'rails/all'

# Require the gems listed in Gemfile, including any gems
# you've limited to :test, :development, or :production.
Bundler.require(*Rails.groups)

require_relative '../lib/med_tracker/version'

# Define namespaces for Phlex components
module Views
end

module Components
  extend Phlex::Kit
end

module RubyUI
  extend Phlex::Kit
end

module MedTracker
  class Application < Rails::Application
    # Initialize configuration defaults for originally generated Rails version.
    config.load_defaults 8.0

    # Please, add to the `ignore` list any other `lib` subdirectories that do
    # not contain `.rb` files, or that should not be reloaded or eager loaded.
    # Common ones are `templates`, `generators`, or `middleware`, for example.
    config.autoload_lib(ignore: %w[assets tasks med_tracker])

    # Configure Phlex autoloading
    Rails.autoloaders.main.push_dir(
      Rails.root.join('app/views'), namespace: Views
    )
    Rails.autoloaders.main.push_dir(
      Rails.root.join('app/components'), namespace: Components
    )

    # Configure RubyUI autoloading
    Rails.autoloaders.main.inflector.inflect('ruby_ui' => 'RubyUI')
    Rails.autoloaders.main.push_dir(
      Rails.root.join('app/components/ruby_ui'), namespace: RubyUI
    )
    Rails.autoloaders.main.collapse(Rails.root.join('app/components/ruby_ui/*'))

    # Configuration for the application, engines, and railties goes here.
    #
    # These settings can be overridden in specific environments using the files
    # in config/environments, which are processed later.
    #
    # config.time_zone = "Central Time (US & Canada)"
    # config.eager_load_paths << Rails.root.join("extras")
  end
end
