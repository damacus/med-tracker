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
        div(class: 'container mx-auto px-4 py-12 max-w-6xl', data: { testid: 'people-list' }) do
          render_header
          render_people_grid
        end
      end

      private

      def render_header
        header(class: 'mb-10 flex flex-col gap-6 md:flex-row md:items-end md:justify-between') do
          div do
            m3_text(size: '2', weight: 'muted', class: 'mb-1 block font-bold uppercase tracking-widest') do
              t('admin.dashboard.quick_actions.manage_people_title')
            end
            m3_heading(level: 1, size: '7', class: 'font-bold tracking-tight') { 'People' }
          end
          if view_context.policy(Person.new).new?
            m3_link(
              href: new_person_path,
              variant: :filled,
              size: :lg,
              class: 'w-full justify-center font-bold text-sm shadow-elevation-2 md:w-auto',
              data: { turbo_frame: 'modal' }
            ) { 'New Person' }
          end
        end
      end

      def render_people_grid
        div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6', id: tenant_dom_target('people')) do
          people.each do |person|
            render PersonCard.new(person: person)
          end
        end
      end
    end
  end
end
