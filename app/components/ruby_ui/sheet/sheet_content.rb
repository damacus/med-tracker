# frozen_string_literal: true

module RubyUI
  class SheetContent < Base
    SIDE_CLASS = {
      top: 'inset-x-0 top-0 border-b data-[state=closed]:slide-out-to-top data-[state=open]:slide-in-from-top',
      right: 'inset-y-0 right-0 h-full border-l data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right',
      bottom: 'inset-x-0 bottom-0 border-t data-[state=closed]:slide-out-to-bottom data-[state=open]:slide-in-from-bottom',
      left: 'inset-y-0 left-0 h-full border-r data-[state=closed]:slide-out-to-left data-[state=open]:slide-in-from-left'
    }.freeze

    def initialize(side: :right, **attrs)
      @side = side
      @side_classes = SIDE_CLASS[side]
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
        class: [
          'fixed pointer-events-auto z-50 gap-4 border-border/70 bg-popover p-6 shadow-elevation-4 transition ease-in-out data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:duration-300 data-[state=open]:duration-500',
          @side_classes
        ]
      }
    end

    def close_button
      button(
        type: 'button',
        class: 'absolute end-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-tertiary-container data-[state=open]:text-on-surface-variant',
        data_action: 'click->ruby-ui--sheet-content#close'
      ) do
        render ::Components::Icons::X.new(size: 16)
        span(class: 'sr-only') { I18n.t('ruby_ui.common.close') }
      end
    end

    def background
      div(
        data_testid: 'drawer-backdrop',
        data_action: 'click->ruby-ui--sheet-content#close',
        class: 'fixed inset-0 z-50 bg-foreground/10 backdrop-blur-[1.5px] transition-opacity duration-300 pointer-events-auto ' \
               'data-[state=open]:opacity-100 data-[state=closed]:opacity-0',
        aria_hidden: 'true'
      )
    end

    def container(&)
      div(
        role: 'dialog',
        aria_modal: 'true',
        aria_label: I18n.t('ruby_ui.common.navigation_menu'),
        tabindex: '-1',
        class: [
          'fixed z-50 flex h-full w-[80vw] max-w-[300px] flex-col overflow-y-auto border-border/70 bg-popover p-6 shadow-elevation-4 transition-transform duration-300 ease-in-out pointer-events-auto',
          'data-[state=open]:translate-x-0',
          side_transform_class
        ],
        &
      )
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
  end
end
