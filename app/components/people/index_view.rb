# frozen_string_literal: true

module Components
  module People
    # People index view component
    class IndexView < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :people

      def initialize(people:)
        @people = people
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 pb-24 md:pb-8', data: { testid: 'people-list' }) do
          render_header
          render_modal_frame
          render_people_grid
        end
      end

      private

      def render_header
        render Components::Shared::PageHeader.new(title: 'People') do |section|
          if section == :actions && view_context.policy(Person.new).create?
            Link(
              href: new_person_path,
              variant: :primary,
              class: 'min-h-[44px]',
              data: { turbo_frame: 'modal' }
            ) { 'New Person' }
          end
        end
      end

      def render_modal_frame
        turbo_frame_tag 'modal'
      end

      def render_people_grid
        div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6', id: 'people') do
          people.each do |person|
            render PersonCard.new(person: person)
          end
        end
      end
    end
  end
end
