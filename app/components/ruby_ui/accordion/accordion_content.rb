# frozen_string_literal: true

module RubyUI
  class AccordionContent < Base
    def initialize(labelledby: nil, open: false, **attrs)
      @labelledby = labelledby
      @open = open
      super(**attrs)
    end

    def view_template(&)
      div(**attrs, &)
    end

    private

    def default_attrs
      {
        role: 'region',
        aria: { labelledby: @labelledby },
        data: {
          ruby_ui__accordion_target: 'content',
          state: @open ? 'open' : 'closed'
        },
        class: 'overflow-y-hidden',
        style: @open ? 'height: auto;' : 'height: 0px;',
        hidden: @open ? nil : true
      }
    end
  end
end
