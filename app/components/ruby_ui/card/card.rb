# frozen_string_literal: true

module RubyUI
  class Card < Base
    def view_template(&)
      div(**attrs, &)
    end

    private

    def default_attrs
      {
        class: 'rounded-[2rem] border bg-background shadow-sm'
      }
    end
  end
end
