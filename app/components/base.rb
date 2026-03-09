# frozen_string_literal: true

module Components
  class Base < Phlex::HTML
    include RubyUI
    # Include any helpers you want to be available across all components
    include Phlex::Rails::Helpers::Routes
    include Phlex::Rails::Helpers::LinkTo
    include Phlex::Rails::Helpers::T
    include Phlex::Rails::Helpers::FormWith
    include Phlex::Rails::Helpers::Pluralize
    include Components::FormHelpers

    def render_version_badge
      div(class: 'version-badge-container mt-auto py-4 px-2 flex justify-center md:justify-start w-full opacity-40 hover:opacity-100 transition-opacity') do
        render RubyUI::Badge.new(variant: :outline, size: :sm) do
          "v#{MedTracker::VERSION}"
        end
      end
    end

    if Rails.env.development?
      def before_template
        comment { "Before #{self.class.name}" }
        super
      end
    end
  end
end
