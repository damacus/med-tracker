# frozen_string_literal: true

module Components
  module Admin
    module People
      class IndexView < Components::Base
        attr_reader :people

        def initialize(people: [])
          @people = people
          super()
        end

        def view_template
          div(data: { testid: 'admin-people' },
              class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl space-y-8') do
            render_header
            people.any? ? render_table : render_empty_state
          end
        end

        private

        def render_header
          header(class: 'space-y-2') do
            m3_heading(level: 1) { t('admin.people.index.title') }
            m3_text(weight: 'muted') { t('admin.people.index.subtitle') }
          end
        end

        def render_table
          div(class: 'rounded-[2rem] border border-border bg-card shadow-sm overflow-x-auto p-4') do
            Table do
              TableHeader(class: 'bg-secondary-container') do
                TableRow do
                  TableHead { t('admin.people.index.table.name') }
                  TableHead { t('admin.people.index.table.type') }
                  TableHead(class: 'text-right') { t('admin.people.index.table.actions') }
                end
              end
              TableBody do
                people.each { |person| render_row(person) }
              end
            end
          end
        end

        def render_row(person)
          TableRow(class: 'hover:bg-tertiary-container', data: { person_id: person.id }) do
            TableCell(class: 'font-semibold text-foreground') { person.name }
            TableCell(class: 'text-on-surface-variant') { person.person_type.to_s.titleize }
            TableCell(class: 'text-right') do
              a(href: "/people/#{person.id}", class: 'text-primary hover:text-primary/80 font-medium') do
                t('admin.people.index.table.view')
              end
            end
          end
        end

        def render_empty_state
          div(data: { testid: 'admin-people-empty' },
              class: 'rounded-xl border border-border bg-card p-12 text-center shadow-sm') do
            m3_text(size: '4', class: 'text-on-surface-variant') { t('admin.people.index.empty') }
          end
        end
      end
    end
  end
end
