# frozen_string_literal: true

module RubyUI
  class InputOtpGroup < Base
    def view_template(&)
      div(**attrs, &)
    end

    private

    def default_attrs
      { class: 'flex items-center' }
    end
  end
end
