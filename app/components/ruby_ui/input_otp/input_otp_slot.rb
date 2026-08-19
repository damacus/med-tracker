# frozen_string_literal: true

module RubyUI
  class InputOtpSlot < Base
    def initialize(index:, **attrs)
      @index = index
      super(**attrs)
    end

    def view_template
      div(**attrs)
    end

    private

    def default_attrs
      {
        aria_hidden: 'true',
        data: {
          ruby_ui__input_otp_target: 'slot',
          index: @index
        },
        class: [
          'relative flex h-11 w-10 items-center justify-center border-y border-r border-input text-base ' \
          'font-bold shadow-xs transition-all sm:h-12 sm:w-12',
          'first:rounded-l-xl first:border-l last:rounded-r-xl',
          'data-[active=true]:z-10 data-[active=true]:border-primary data-[active=true]:ring-2 ' \
          'data-[active=true]:ring-primary/30',
          "data-[caret=true]:after:content-[''] data-[caret=true]:after:absolute data-[caret=true]:after:h-5 " \
          'data-[caret=true]:after:w-px data-[caret=true]:after:animate-caret-blink ' \
          'data-[caret=true]:after:bg-foreground',
          'aria-invalid:border-error data-[active=true]:aria-invalid:border-error ' \
          'data-[active=true]:aria-invalid:ring-error/20'
        ]
      }
    end
  end
end
