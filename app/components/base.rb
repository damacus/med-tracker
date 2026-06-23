# frozen_string_literal: true

module Components
  class Base < Phlex::HTML
    include RubyUI
    include Components::M3Helpers
    # Include any helpers you want to be available across all components
    include Phlex::Rails::Helpers::Routes
    include Phlex::Rails::Helpers::LinkTo
    include Phlex::Rails::Helpers::T
    include Phlex::Rails::Helpers::FormWith
    include Phlex::Rails::Helpers::Pluralize
    include Components::FormHelpers
    include TenantDomTargetsHelper

    if Rails.env.development?
      def before_template
        comment { "Before #{self.class.name}" }
        super
      end
    end

    def policy_context(fallback = nil)
      AuthorizationContext.current || fallback
    end
  end
end
