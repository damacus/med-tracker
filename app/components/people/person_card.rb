# frozen_string_literal: true

module Components
  module People
    # Person card component
    class PersonCard < Components::Base
      attr_reader :person

      def initialize(person:)
        @person = person
        super()
      end

      def view_template
        m3_card(id: "person_#{person.id}", class: 'h-full flex flex-col') do
          render_card_header
          render_card_content
          render_card_footer
        end
      end

      private

      def render_card_header
        m3_card_header(class: 'pb-4') do
          div(class: 'flex items-start justify-between') do
            render_person_icon
            render_needs_carer_badge if person.needs_carer?
          end
          div(class: 'space-y-1 mt-4') do
            m3_link(
              href: person_path(person),
              variant: :text,
              class: 'p-0 h-auto no-underline hover:no-underline'
            ) do
              m3_heading(variant: :title_large, level: 2, class: 'font-black tracking-tight text-foreground hover:text-primary transition-colors') do
                person.name
              end
            end
          end
        end
      end

      def render_person_icon
        div(
          class: 'w-10 h-10 rounded-xl flex items-center justify-center ' \
                 'bg-secondary-container text-on-secondary-container'
        ) do
          render Icons::User.new(size: 20)
        end
      end

      def render_card_content
        m3_card_content(class: 'flex-grow space-y-2') do
          div(class: 'space-y-1.5 text-sm text-on-surface-variant font-medium') do
            p do
              strong(class: 'font-black uppercase tracking-widest text-[10px] opacity-60 mr-2') { "#{t('people.card.born')} " }
              plain person.date_of_birth.strftime('%B %d, %Y')
            end
            p do
              strong(class: 'font-black uppercase tracking-widest text-[10px] opacity-60 mr-2') { "#{t('people.card.age')} " }
              plain person.age.to_s
            end
            p do
              strong(class: 'font-black uppercase tracking-widest text-[10px] opacity-60 mr-2') { "#{t('people.card.medications')} " }
              plain active_medication_count_text
            end
          end
        end
      end

      def active_schedules
        @active_schedules ||= if person.schedules.loaded?
                                person.schedules.select(&:active?)
                              else
                                person.schedules.active
                              end
      end

      def visible_person_medications
        @visible_person_medications ||= person.person_medications.select do |person_medication|
          view_context.policy(person_medication).show?
        end
      end

      def active_medications_count
        active_schedules.size + visible_person_medications.size
      end

      def active_medication_count_text
        if active_medications_count.positive?
          view_context.pluralize(active_medications_count, 'active medication')
        else
          t('people.card.no_active_medications')
        end
      end

      def render_needs_carer_badge
        m3_badge(
          variant: :outlined,
          class: 'h-auto py-1.5 px-3 rounded-xl border-warning/50 bg-warning-container/20 text-on-warning-container ' \
                 'font-black uppercase tracking-wider text-[10px]',
          data: { testid: 'needs-carer-badge' }
        ) do
          plain t('people.card.needs_carer')
        end
      end

      def render_card_footer
        m3_card_footer(class: 'flex flex-col gap-3 mt-auto pt-6') do
          render_add_medication_link if can_add_medication?

          div(class: 'flex gap-2 w-full') do
            if active_medications_count.positive?
              m3_link(
                href: person_path(person),
                variant: :outlined,
                size: :lg,
                class: 'flex-1 rounded-xl font-bold bg-surface-container-low'
              ) { t('people.card.view_medications') }
            end

            render_assign_carer_link if person.needs_carer? && can_create?(CarerRelationship)
          end
        end
      end

      def render_add_medication_link
        m3_link(
          href: add_medication_person_path(person),
          variant: :filled,
          size: :lg,
          class: 'w-full rounded-xl font-black py-6 shadow-lg shadow-primary/20'
        ) { t('people.card.add_medication') }
      end

      def can_add_medication?
        can_create?(Schedule.new(person: person)) ||
          can_create?(PersonMedication.new(person: person))
      end

      def render_assign_carer_link
        m3_link(
          href: new_admin_carer_relationship_path(patient_id: person.id),
          variant: :outlined,
          size: :lg,
          class: 'flex-1 rounded-xl font-bold text-on-warning-container border-warning/50 bg-warning-container/10',
          data: { turbo_frame: 'modal' }
        ) { t('people.card.assign_carer') }
      end

      def can_create?(record)
        return false unless view_context.respond_to?(:policy)

        view_context.policy(record).create?
      end
    end
  end
end