# frozen_string_literal: true

module Views
  module Layouts
    # Modal layout that wraps content in a dialog element and shows it automatically.
    class Modal < Views::Base
      def view_template(&block)
        turbo_frame_tag 'modal' do
          dialog id: 'dialog' do
            block&.call
          end

          script type: 'text/javascript' do
            plain 'dialog.showModal();'
          end
        end
      end
    end
  end
end
