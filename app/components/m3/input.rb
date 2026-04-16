# frozen_string_literal: true

module Components
  module M3
  class Input < RubyUI::Input
    private

    def default_attrs
      {
        data: {
          ruby_ui__form_field_target: 'input',
          action: 'input->ruby-ui--form-field#onInput invalid->ruby-ui--form-field#onInvalid'
        },
        class: [
          'flex h-14 min-h-[56px] w-full rounded-shape-xs border bg-transparent px-4 py-4 text-base transition-all border-outline',
          'placeholder:text-on-surface-variant',
          'disabled:cursor-not-allowed disabled:opacity-38',
          'file:border-0 file:bg-transparent file:text-sm file:font-medium',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary',
          'aria-disabled:cursor-not-allowed aria-disabled:opacity-38 aria-disabled:pointer-events-none'
        ]
      }
    end
  end
  end
end
