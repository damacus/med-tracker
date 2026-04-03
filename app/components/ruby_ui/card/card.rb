# frozen_string_literal: true

module RubyUI
  class Card < Base
    def view_template(&)
      div(**attrs, &)
    end

    private

    def default_attrs
      {
        class: 'rounded-shape-xl border bg-surface-container-low shadow-elevation-1'
      }
    end
  end
end
