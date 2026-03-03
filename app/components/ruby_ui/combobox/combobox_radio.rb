# frozen_string_literal: true

module RubyUI
  class ComboboxRadio < Base
    def view_template
      input(type: 'radio', **attrs)
    end

    private

    def default_attrs
      {
        class: [
          'appearance-none absolute h-0 w-0 opacity-0',
          'focus:outline-none',
          'disabled:cursor-not-allowed disabled:opacity-50',
          'aria-disabled:cursor-not-allowed aria-disabled:opacity-50 aria-disabled:pointer-events-none'
        ],
        data: {
          ruby_ui__combobox_target: 'input',
          ruby_ui__form_field_target: 'input',
          action: %w[
            ruby-ui--combobox#inputChanged
            input->ruby-ui--form-field#onInput
            invalid->ruby-ui--form-field#onInvalid
          ]
        }
      }
    end
  end
end
