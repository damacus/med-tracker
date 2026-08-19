# frozen_string_literal: true

module RubyUI
  class SheetContent < Base
    SIZES = {
      sm: 'max-w-sm',
      md: 'max-w-md',
      lg: 'max-w-lg',
      xl: 'max-w-xl',
      full: 'max-w-none'
    }.freeze

    def initialize(side: :right, size: :md, show_close: true, **attrs)
      @side = side
      @size = size
      @show_close = show_close
      super(**attrs)
    end

    def view_template(&)
      template(data: { 'ruby-ui--sheet-target': 'content' }) do
        background
        container(&)
      end
    end

    private

    def default_attrs
      {
        data_state: 'open', # For animate in
        role: 'dialog',
        aria_modal: 'true',
        tabindex: '-1',
        class: [
          'fixed z-50 flex h-full w-[calc(100vw-2rem)] flex-col overflow-y-auto border-border/70 bg-popover p-6 ' \
          'shadow-elevation-4 transition-transform duration-300 ease-in-out pointer-events-auto',
          size_class,
          'data-[state=open]:translate-x-0',
          side_transform_class
        ]
      }
    end

    def close_button
      button(
        type: 'button',
        class: 'absolute end-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-tertiary-container data-[state=open]:text-on-surface-variant',
        data_action: 'click->ruby-ui--sheet-content#close',
        aria: { label: I18n.t('ruby_ui.common.close') }
      ) do
        render ::Components::Icons::X.new(size: 16, aria_hidden: 'true')
      end
    end

    def background
      div(
        data_testid: 'drawer-backdrop',
        data_action: 'click->ruby-ui--sheet-content#close',
        class: 'fixed inset-0 z-50 bg-foreground/10 backdrop-blur-[1.5px] transition-opacity duration-300 pointer-events-auto ' \
               'data-[state=open]:opacity-100 data-[state=closed]:opacity-0 data-[state=closed]:pointer-events-none',
        aria_hidden: 'true',
        data_state: 'closed'
      )
    end

    def container(&block)
      div(**attrs) do
        block&.call
        close_button if @show_close
      end
    end

    def side_transform_class
      case @side
      when :left
        'top-0 left-0 border-r data-[state=closed]:-translate-x-full'
      when :right
        'top-0 right-0 border-l data-[state=closed]:translate-x-full'
      when :top
        'top-0 left-0 w-full h-auto border-b data-[state=closed]:-translate-y-full'
      when :bottom
        'bottom-0 left-0 w-full h-auto border-t data-[state=closed]:translate-y-full'
      end
    end

    def size_class
      return 'max-w-none' if %i[top bottom].include?(@side)

      SIZES.fetch(@size)
    end
  end
end
