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
      template(data: { ruby_ui__dialog_target: 'content' }) do
        div(data_controller: 'ruby-ui--dialog') do
          backdrop
          div(**attrs) do
            yield
            close_button
          end
        end
      end
    end

    private

    def default_attrs
      {
        data_state: 'open',
        class: [
          'fixed flex flex-col pointer-events-auto left-[50%] top-[50%] z-50 w-full max-h-screen overflow-y-auto translate-x-[-50%] translate-y-[-50%] gap-4 border bg-background p-6 shadow-lg duration-200 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 sm:rounded-lg md:w-full',
          SIZES[@size]
        ]
      }
    end

    def close_button
      button(
        type: 'button',
        class: 'absolute end-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-accent data-[state=open]:text-muted-foreground',
        data_action: 'click->ruby-ui--dialog#dismiss'
      ) do
        render ::Components::Icons::X.new(size: 16)
        span(class: 'sr-only') { 'Close' }
      end
    end

    def backdrop
      div(
        data_state: 'open',
        data_action: 'click->ruby-ui--dialog#dismiss esc->ruby-ui--dialog#dismiss',
        class: 'fixed pointer-events-auto inset-0 z-50 bg-background/80 backdrop-blur-sm data-[state=open]:animate-in data-[state=open]:fade-in-0'
      )
    end
  end
end
