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
        div(class: "container mx-auto px-4 py-12 max-w-6xl", data: {testid: "people-list"}) do
          render_header
          render_people_grid
        end
      end

      private

      def render_header
        div(class: "mb-10 flex flex-col gap-4 md:flex-row md:items-center md:justify-between") do
          m3_heading(level: 1) { "People" }
          if view_context.policy(Person.new).new?
            div(class: "hidden md:block") do
              Link(href: new_person_path, variant: :primary, data: {turbo_frame: "modal"}) { "New Person" }
            end
          end
        end
      end

      def render_people_grid
        div(class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6", id: "people") do
          people.each do |person|
            render(PersonCard.new(person: person))
          end
        end
      end
    end
  end
end
