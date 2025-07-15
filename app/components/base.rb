# frozen_string_literal: true

module Components
  # The base class for all components.
  class Base < Phlex::HTML
    # Include any helpers you want to be available across all components
    include Phlex::Rails::Helpers::Routes
    include Phlex::Rails::Helpers::LinkTo
    include Phlex::Rails::Helpers::TurboFrameTag
    include Phlex::Rails::Helpers::FormWith
    include ActionView::Helpers::FormOptionsHelper
    include ActionView::Helpers::TranslationHelper

    if Rails.env.development?
      def before_template
        comment { "Before #{self.class.name}" }
        super
      end
    end
  end
end
