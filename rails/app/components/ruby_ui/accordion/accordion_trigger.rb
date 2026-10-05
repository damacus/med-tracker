# frozen_string_literal: true

module RubyUI
  class AccordionTrigger < Base
    def initialize(controls: nil, expanded: false, **attrs)
      @controls = controls
      @expanded = expanded
      super(**attrs)
    end

    def view_template(&)
      button(**attrs, &)
    end

    private

    def default_attrs
      {
        type: 'button',
        aria: { controls: @controls, expanded: @expanded.to_s },
        data: {
          action: 'click->ruby-ui--accordion#toggle',
          ruby_ui__accordion_target: 'trigger'
        },
        class: 'flex w-full flex-1 items-center justify-between py-4 text-sm font-medium transition-all'
      }
    end
  end
end
