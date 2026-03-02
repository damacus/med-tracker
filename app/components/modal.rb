# frozen_string_literal: true

module Components
  class Modal < ::RubyUI::Base
    include RubyUI
    # Include necessary helpers from Components::Base
    include Phlex::Rails::Helpers::Routes
    include Phlex::Rails::Helpers::LinkTo
    include Phlex::Rails::Helpers::T
    include Components::FormHelpers

    SIZES = {
      xs: 'max-w-sm',
      sm: 'max-w-md',
      md: 'max-w-lg',
      lg: 'max-w-2xl',
      xl: 'max-w-4xl',
      full: 'max-w-full'
    }.freeze

    def initialize(title: nil, subtitle: nil, size: :md, **attrs)
      @title = title
      @subtitle = subtitle
      @size = size
      super(**attrs)
    end

    def view_template(&block)
      dialog(
        **attrs,
        open: true,
        class: [
          'fixed inset-0 z-50 m-auto flex flex-col p-0 bg-transparent',
          'backdrop:bg-background/80 backdrop:backdrop-blur-sm',
          'open:animate-in open:fade-in-0 open:zoom-in-95',
          attrs[:class]
        ]
      ) do
        div(
          class: [
            'relative w-full bg-background border shadow-lg sm:rounded-lg overflow-hidden flex flex-col',
            SIZES[@size]
          ]
        ) do
          render_header if @title || @subtitle
          div(class: 'p-6 overflow-y-auto max-h-[80vh]') { block.call if block_given? }
          close_button
        end
      end
    end

    private

    def default_attrs
      {
        data: {
          controller: 'modal',
          action: 'click->modal#closeOnBackdropClick'
        }
      }
    end

    def render_header
      div(class: 'p-6 border-b space-y-1.5') do
        Heading(level: 2, size: '6', class: 'font-semibold leading-none tracking-tight') { @title } if @title
        Text(weight: 'muted', size: '2') { @subtitle } if @subtitle
      end
    end

    def close_button
      button(
        type: 'button',
        class: 'absolute right-4 top-4 rounded-sm opacity-70 transition-opacity ' \
               'hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring ' \
               'focus:ring-offset-2',
        data_action: 'click->modal#close'
      ) do
        render ::Components::Icons::X.new(size: 16)
        span(class: 'sr-only') { 'Close' }
      end
    end
  end
end
