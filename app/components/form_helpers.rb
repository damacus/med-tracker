# frozen_string_literal: true

module Components
  # Shared helper methods for form components
  # Include this module in components that need consistent form styling
  module FormHelpers
    extend ActiveSupport::Concern

    # Standard styling for native select elements to match RubyUI components
    # Use this when RubyUI::Select is not compatible with Capybara tests
    def select_classes
      'flex h-9 w-full items-center justify-between rounded-md border border-input ' \
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

    def render_field_error(model, field)
      return unless model.errors[field].any?

      FormFieldError { model.errors[field].first }
    end

    def render_input_field(label:, name:, id:, **opts)
      model = opts.delete(:model)
      field = opts.delete(:field)
      opts[:type] ||= :text
      opts[:class] = model && field ? field_error_class(model, field) : ''

      FormField do
        FormFieldLabel(for: id) { label }
        Input(name: name, id: id, **opts)
        render_field_error(model, field) if model && field
      end
    end

    def render_textarea_field(label:, name:, id:, value: nil, rows: 3)
      FormField do
        FormFieldLabel(for: id) { label }
        Textarea(name: name, id: id, rows: rows) { value }
      end
    end
  end
end
