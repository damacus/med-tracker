# frozen_string_literal: true

module RubyUI
  class AccordionIcon < Base
    def view_template(&block)
      span(**attrs) do
        block ? block.call : icon
      end
    end

    private

    def icon
      render Components::Icons::ArrowDown.new(size: 18, aria_hidden: 'true')
    end

    def default_attrs
      {
        aria: { hidden: 'true' },
        class: 'shrink-0 text-on-surface-variant',
        data: { ruby_ui__accordion_target: 'icon' }
      }
    end
  end
end
