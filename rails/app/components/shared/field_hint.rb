# frozen_string_literal: true

module Components
  module Shared
    class FieldHint < Components::Base
      def view_template(&)
        render RubyUI::FormFieldHint.new(
          class: 'text-xs text-on-surface-variant font-medium ml-1 mb-1'
        ), &
      end
    end
  end
end
