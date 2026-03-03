# frozen_string_literal: true

module Components
  module People
    # Modal wrapper for the person create/edit form
    class Modal < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :person, :title, :subtitle, :return_to

      def initialize(person:, title: nil, subtitle: nil, return_to: nil)
        @person = person
        @title = title || (person.new_record? ? 'New Person' : 'Edit Person')
        @subtitle = subtitle || (person.new_record? ? 'Add a new person to track medications for' : "Update #{person.name}'s details")
        @return_to = return_to
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          Dialog(open: true) do
            DialogContent(size: :xl) do
              DialogHeader do
                DialogTitle { title }
                DialogDescription { subtitle }
              end
              DialogMiddle do
                render FormView.new(person: person, return_to: return_to, is_modal: true)
              end
            end
          end
        end
      end
    end
  end
end
