# frozen_string_literal: true

module Components
  # Shared helper methods for form components
  # Include this module in components that need consistent form styling
  module FormHelpers
    extend ActiveSupport::Concern

    # Standard styling for native select elements to match RubyUI components
    # Use this when RubyUI::Select is not compatible with Capybara tests
    def select_classes
      'block h-9 w-full rounded-md border border-outline ' \
        'bg-transparent px-3 py-2 text-sm shadow-sm ring-offset-background ' \
        'focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50'
    end

    # Standard styling for checkbox inputs
    # Matches the design system with proper focus states and sizing
    def checkbox_classes
      'h-4 w-4 shrink-0 rounded border border-primary shadow ' \
        'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring ' \
        'disabled:cursor-not-allowed disabled:opacity-50 ' \
        'checked:bg-primary checked:text-primary-foreground'
    end

    def field_error_class(model, field)
      model.errors[field].any? ? 'border-destructive focus-visible:ring-destructive' : ''
    end

    def field_error_attributes(model, field, input_id: nil)
      return {} unless model.errors[field].any?

      attributes = { aria: { invalid: true } }
      attributes[:aria][:describedby] = field_error_id(input_id) if input_id.present?
      attributes
    end

    def render_field_error(model, field, input_id: nil)
      return unless model.errors[field].any?

      error_attributes = {}
      error_attributes[:id] = field_error_id(input_id) if input_id.present?
      FormFieldError(**error_attributes) { model.errors[field].first }
    end

    def field_error_id(input_id)
      "#{input_id}_error"
    end
  end
end
