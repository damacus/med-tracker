# frozen_string_literal: true

module RubyUI
  class AlertDialogAction < Base
    def view_template(&)
      Button(**attrs, &)
    end

    private

    def default_attrs
      {
        variant: :primary
      }
    end
  end
end
