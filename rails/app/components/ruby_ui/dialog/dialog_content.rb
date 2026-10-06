# frozen_string_literal: true

module RubyUI
  class DialogContent < Base
    SIZES = {
      xs: 'max-w-sm',
      sm: 'max-w-md',
      md: 'max-w-lg',
      lg: 'max-w-2xl',
      xl: 'max-w-4xl',
      full: 'max-w-full'
    }.freeze

    def initialize(size: :md, **attrs)
      @size = size
      super(**attrs)
    end

    def view_template
      dialog(
        role: 'dialog',
        aria: {
          modal: 'true'
        },
        tabindex: '-1',
        hidden: true,
        data: {
          ruby_ui__dialog_target: 'content',
          action: 'cancel->ruby-ui--dialog#dismiss click->ruby-ui--dialog#backdropClick ' \
                  'keydown->ruby-ui--dialog#trapFocus'
        },
        **attrs
      ) do
        yield
        close_button
      end
    end

    private

    def default_attrs
      {
        data_state: 'open',
        class: [
          'fixed flex flex-col left-[50%] top-[50%] z-50 w-full max-h-screen overflow-y-auto translate-x-[-50%] translate-y-[-50%] border border-border/70 bg-popover text-foreground shadow-elevation-5 duration-200 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 sm:rounded-shape-xl md:w-full backdrop:bg-foreground/10 backdrop:backdrop-blur-[1.5px]',
          SIZES[@size]
        ]
      }
    end

    def close_button
      button(
        type: 'button',
        class: 'absolute end-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-tertiary-container data-[state=open]:text-on-surface-variant',
        data_action: 'click->ruby-ui--dialog#dismiss',
        aria: { label: I18n.t('ruby_ui.common.close') }
      ) do
        render ::Components::Icons::X.new(size: 16, aria_hidden: 'true')
      end
    end
  end
end
