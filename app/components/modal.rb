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

    def initialize(title: nil, subtitle: nil, size: :md, **attrs)
      @title = title
      @subtitle = subtitle
      @size = size
      super(**attrs)
    end

    def view_template(&block)
      content_attrs = attrs.except(:class)

      Dialog(open: true) do
        DialogContent(
          **content_attrs,
          size: @size,
          class: [
            'border-outline-variant bg-surface-container-high shadow-elevation-5 ' \
            'sm:rounded-[2.5rem] overflow-hidden',
            attrs[:class]
          ]
        ) do
          render_header if @title || @subtitle
          DialogMiddle(class: 'p-8 overflow-y-auto') { block.call if block_given? }
        end
      end
    end

    private

    def render_header
      DialogHeader(class: 'px-8 pt-8 pb-4 border-b border-outline-variant/30 space-y-1.5') do
        DialogTitle(class: 'text-2xl font-black tracking-tight') { @title } if @title
        DialogDescription(class: 'text-on-surface-variant font-medium') { @subtitle } if @subtitle
      end
    end
  end
end
