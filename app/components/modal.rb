# frozen_string_literal: true

module Components
  class Modal < ::RubyUI::Base
    include RubyUI
    include Components::M3Helpers
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
            'relative w-full bg-surface-container-high border-outline-variant shadow-elevation-5 sm:rounded-[2.5rem] overflow-hidden flex flex-col',
            SIZES[@size]
          ]
        ) do
          render_header if @title || @subtitle
          div(class: 'p-8 overflow-y-auto max-h-[80vh]') { block.call if block_given? }
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
      div(class: 'px-8 pt-8 pb-4 border-b border-outline-variant/30 space-y-1.5') do
        m3_heading(variant: :headline_small, level: 2, class: 'font-black tracking-tight') { @title } if @title
        m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') { @subtitle } if @subtitle
      end
    end

    def close_button
      a(
        href: '#',
        class: 'absolute right-6 top-6 flex h-10 w-10 items-center justify-center rounded-full ' \
               'border border-outline-variant/30 bg-surface-container-highest/90 text-on-surface-variant ' \
               'shadow-elevation-1 transition-all hover:bg-secondary-container hover:text-on-secondary-container',
        data_action: 'click->modal#close',
        aria_label: 'Close'
      ) do
        render ::Components::Icons::X.new(size: 18)
        span(class: 'sr-only') { 'Close' }
      end
    end
  end
end
