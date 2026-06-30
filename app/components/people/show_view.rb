# frozen_string_literal: true

module Components
  module People
    # Person show view component
    class ShowView < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include Phlex::Rails::Helpers::FormWith

      attr_reader :person, :schedules, :person_medications, :current_user

      def initialize(person:, schedules:, person_medications: nil, **opts)
        @person = person
        @schedules = schedules
        @person_medications = person_medications || person.person_medications
        @current_user = opts[:current_user]
        super()
      end

      def view_template
        div(
          id: tenant_dom_target("person_show_#{person.id}"),
          class: 'container mx-auto px-4 py-12 max-w-6xl space-y-12'
        ) do
          render_person_header

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-12') do
            div(class: 'lg:col-span-2 space-y-12') do
              render_medications_section
            end

            div(class: 'space-y-8') do
              render_person_overview_card
              render_quick_actions_card
            end
          end
        end
      end

      private

      def render_person_header
        div(
          class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b ' \
                 'border-outline-variant/30'
        ) do
          div(class: 'flex items-center gap-6') do
            render Components::Shared::PersonAvatar.new(person: person, size: :xl)
            div(class: 'space-y-1') do
              div(class: 'flex items-center gap-3') do
                m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight') { person.name }
                m3_badge(variant: :outlined,
                         class: 'rounded-full uppercase text-[10px] font-black tracking-widest py-1 px-3') do
                  person.person_type.humanize
                end
              end
              m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') do
                "#{t('people.show.age')} #{person.age}"
              end
            end
          end

          div(class: 'flex gap-3') do
            if view_context.policy(person).update?
              m3_link(href: person_path(person, editing: true), variant: :outlined, size: :lg,
                      class: 'font-bold bg-surface-container-low transition-all') do
                t('people.show.edit_person')
              end
            end
            m3_link(href: people_path, variant: :text, size: :lg,
                    class: 'font-bold text-on-surface-variant hover:text-foreground') do
              t('people.show.back')
            end
          end
        end
      end

      def render_person_overview_card
        m3_card(variant: :elevated, class: 'p-8 space-y-6 border-none shadow-elevation-1 rounded-[2.5rem]') do
          m3_heading(variant: :title_large, level: 2, class: 'font-bold') { t('people.overview.title') }

          div(class: 'space-y-4') do
            overview_item(t('people.overview.dob'), person.date_of_birth.strftime('%B %d, %Y'), Icons::CheckCircle)
            overview_item(t('people.overview.assigned_user'),
                          person.user&.email_address || t('people.overview.no_user'), Icons::User)
            overview_item(
              t('people.overview.capacity'),
              person.has_capacity ? t('people.overview.has_capacity') : t('people.overview.dependent'),
              Icons::Key
            )
          end
        end
      end

      def overview_item(label, value, icon_class)
        div(class: 'flex items-center gap-4 group') do
          div(
            class: 'w-10 h-10 rounded-xl bg-secondary-container flex items-center ' \
                   'justify-center text-on-secondary-container ' \
                   'group-hover:bg-primary/10 group-hover:text-primary transition-all'
          ) do
            render icon_class.new(size: 20)
          end
          div do
            m3_text(variant: :label_small, class: 'uppercase tracking-widest text-on-surface-variant font-black') do
              label
            end
            m3_text(variant: :body_medium, class: 'font-bold') { value }
          end
        end
      end

      def render_quick_actions_card
        m3_card(
          variant: :filled,
          class: 'bg-primary p-8 text-on-primary border-none shadow-xl shadow-primary/20 ' \
                 'rounded-[2.5rem]',
          data: { testid: 'quick-actions' }
        ) do
          div(class: 'space-y-6') do
            div do
              m3_heading(variant: :headline_small, level: 3, class: 'font-bold mb-2') { t('people.actions.title') }
              m3_text(variant: :body_medium, class: 'text-on-primary/80 font-medium') do
                t('people.actions.subtitle')
              end
            end

            div(class: 'space-y-3') do
              if can_add_medication?
                m3_link(
                  href: new_person_medication_assignment_path(person),
                  variant: :tonal,
                  size: :lg,
                  class: 'w-full font-bold shadow-elevation-1 transition-all',
                  data: { turbo_frame: 'modal' }
                ) { t('people.show.add_medication') }
              end
              if can_assign_carer?
                m3_link(
                  href: new_person_carer_relationship_path(person),
                  variant: :tonal,
                  size: :lg,
                  class: 'w-full font-bold shadow-elevation-1 transition-all'
                ) { t('people.show.manage_parents') }
              end
            end
          end
        end
      end

      def render_medications_section
        div(class: 'space-y-8') do
          div(class: 'flex items-center justify-between px-2') do
            m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') do
              t('people.show.medications_heading')
            end
            div(class: 'h-1 flex-1 mx-8 bg-outline-variant/20 rounded-full hidden md:block')
          end

          div(id: 'medications', class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
            medication_sources.each { |source| render_medication_source(source) }
          end

          render_empty_state if medication_sources.none?
        end
      end

      def medication_sources
        @medication_sources ||= (schedules.to_a + accessible_person_medications).sort_by do |source|
          source.paused? ? 1 : 0
        end
      end

      def accessible_person_medications
        @accessible_person_medications ||= person_medications.select { |pm| view_context.policy(pm).show? }
      end

      def render_medication_source(source)
        if source.is_a?(Schedule)
          render Components::Schedules::Card.new(
            schedule: source,
            person: person,
            current_user: current_user
          )
        else
          render Components::PersonMedications::Card.new(
            person_medication: source,
            person: person,
            current_user: current_user
          )
        end
      end

      def render_empty_state
        div(class: 'col-span-full') do
          m3_card(variant: :filled,
                  class: 'text-center py-16 px-8 border-dashed border-2 border-outline-variant/50 ' \
                         'bg-surface-container-low rounded-[2.5rem]') do
            m3_text(variant: :body_large, class: 'text-on-surface-variant mb-6 font-medium italic') do
              t('people.show.no_any_medications')
            end
            if can_add_medication?
              m3_link(
                href: new_person_medication_assignment_path(person),
                variant: :filled,
                size: :lg,
                data: { turbo_frame: 'modal' }
              ) { t('people.show.add_first_any_medication') }
            end
          end
        end
      end

      def can_create_schedule?
        view_context.policy(Schedule.new(person: person)).create?
      end

      def can_add_medication?
        can_create_schedule? || view_context.policy(PersonMedication.new(person: person)).create?
      end

      def can_assign_carer?
        view_context.policy(CarerRelationship.new(patient: person)).assign_dependent?
      end
    end
  end
end
