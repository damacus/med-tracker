# frozen_string_literal: true

module RubyUI
  class FormFieldError < Base
    def view_template(&)
      p(**attrs, &)
    end

    private

    def default_attrs
      {
        data: {
          ruby_ui__form_field_target: 'error'
        },
        class: 'text-sm font-medium text-destructive',
        role: 'alert'
      }
    end
  end
end
