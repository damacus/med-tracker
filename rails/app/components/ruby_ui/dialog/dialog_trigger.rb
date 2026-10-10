# frozen_string_literal: true

module RubyUI
  class DialogTrigger < Base
    def view_template(&)
      div(**attrs, &)
    end

    private

    def default_attrs
      {
        inert: true,
        data: {
          action: 'click->ruby-ui--dialog#open',
          ruby_ui__dialog_target: 'trigger',
          ruby_ui_overlay_trigger: true
        },
        class: 'inline-block'
      }
    end
  end
end
