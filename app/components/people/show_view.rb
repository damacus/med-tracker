# frozen_string_literal: true

module Components
  module People
    # Person show view component
    class ShowView < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include Phlex::Rails::Helpers::FormWith

      attr_reader :person, :schedules, :person_medications, :editing,
                  :takes_by_schedule, :takes_by_person_medication, :current_user

      def initialize(person:, schedules:, person_medications: nil, editing: false, **opts)
        @person = person
        @schedules = schedules
        @person_medications = person_medications || person.person_medications
        @editing = editing
        preloaded_takes = opts.fetch(:preloaded_takes, {})
        @takes_by_schedule = preloaded_takes.fetch(:schedules, {})
        @takes_by_person_medication = preloaded_takes.fetch(:person_medications, {})
        @current_user = opts[:current_user]
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-6xl space-y-12') do
          render_person_header

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-12') do
            div(class: 'lg:col-span-2 space-y-12') do
              render_schedules_section
              render_my_medications_section
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
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b border-slate-100') do
          div(class: 'flex items-center gap-6') do
            div(
              class: 'w-24 h-24 rounded-[2rem] bg-primary/10 flex items-center justify-center text-primary ' \
                     'font-black text-3xl shadow-inner'
            ) do
              person.name.split.map(&:first).join.upcase
            end
            div(class: 'space-y-1') do
              div(class: 'flex items-center gap-3') do
                Heading(level: 1, size: '8', class: 'font-black tracking-tight') { person.name }
                Badge(variant: :outline, class: 'rounded-full uppercase text-[10px] tracking-widest py-1 px-3') do
                  person.person_type.humanize
                end
              end
              Text(size: '4', weight: 'muted') { "#{t('people.show.age')} #{person.age}" }
            end
          end

          div(class: 'flex gap-3') do
            if view_context.policy(person).update?
              Link(href: person_path(person, editing: true), variant: :outline, size: :lg,
                   class: 'rounded-2xl font-bold text-sm bg-white') do
                t('people.show.edit_person')
              end
            end
            Link(href: people_path, variant: :ghost, size: :lg,
                 class: 'rounded-2xl font-bold text-sm text-slate-400 hover:text-slate-600') do
              t('people.show.back')
            end
          end
        end
      end

      def render_person_overview_card
        Card(class: 'p-8 space-y-6') do
          Heading(level: 2, size: '4', class: 'font-bold') { t('people.overview.title') }

          div(class: 'space-y-4') do
            overview_item(t('people.overview.dob'), person.date_of_birth.strftime('%B %d, %Y'), Icons::CheckCircle)
            overview_item(t('people.overview.assigned_user'),
                          person.user&.email_address || t('people.overview.no_user'), Icons::User)
            overview_item(t('people.overview.capacity'),
                          person.has_capacity ? t('people.overview.has_capacity') : t('people.overview.dependent'), Icons::Key)
          end
        end
      end

      def overview_item(label, value, icon_class)
        div(class: 'flex items-center gap-4 group') do
          div(
            class: 'w-10 h-10 rounded-xl bg-slate-50 flex items-center justify-center text-slate-400 ' \
                   'group-hover:bg-primary/5 group-hover:text-primary transition-colors'
          ) do
            render icon_class.new(size: 20)
          end
          div do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-400') { label }
            Text(size: '2', weight: 'semibold') { value }
          end
        end
      end

      def render_quick_actions_card
        Card(class: 'bg-primary p-8 text-white border-none shadow-xl shadow-primary/20') do
          div(class: 'space-y-6') do
            div do
              Heading(level: 3, size: '5', class: 'font-bold mb-2') { t('people.actions.title') }
              Text(size: '2', class: 'text-primary-foreground opacity-80') do
                t('people.actions.subtitle')
              end
            end

            div(class: 'space-y-3') do
              if can_create_schedule?
                Link(
                  href: new_person_schedule_path(person),
                  variant: :secondary,
                  class: 'w-full py-6 rounded-xl font-bold text-sm bg-white text-primary border-none shadow-sm',
                  data: { turbo_stream: true }
                ) { t('people.show.add_schedule') }
              end

              if view_context.policy(PersonMedication.new(person: person)).create?
                Link(
                  href: new_person_person_medication_path(person),
                  variant: :outline,
                  class: 'w-full py-6 rounded-xl font-bold text-sm bg-primary-foreground/10 text-white ' \
                         'border-white/20 hover:bg-primary-foreground/20',
                  data: { turbo_stream: true }
                ) { t('people.show.add_medication') }
              end
            end
          end
        end
      end

      def render_schedules_section
        div(class: 'space-y-8') do
          div(class: 'flex items-center justify-between px-2') do
            Heading(level: 2, size: '6', class: 'font-bold tracking-tight') { t('people.show.schedules_heading') }
            div(class: 'h-1 flex-1 mx-8 bg-slate-50 rounded-full hidden md:block')
          end

          turbo_frame_tag 'schedule_modal'

          div(id: 'schedules', class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
            if schedules.any?
              schedules.each do |schedule|
                render Components::Schedules::Card.new(
                  schedule: schedule,
                  person: person,
                  todays_takes: takes_by_schedule[schedule.id],
                  current_user: current_user
                )
              end
            else
              render_empty_state
            end
          end
        end
      end

      def render_my_medications_section
        div(class: 'space-y-8') do
          div(class: 'flex items-center justify-between px-2') do
            Heading(level: 2, size: '6', class: 'font-bold tracking-tight') { t('people.show.my_medications_heading') }
            div(class: 'h-1 flex-1 mx-8 bg-slate-50 rounded-full hidden md:block')
          end

          turbo_frame_tag 'person_medication_modal'

          # Filter person_medications based on policy
          accessible_medications = person_medications.select do |pm|
            view_context.policy(pm).show?
          end

          div(id: 'person_medications', class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
            if accessible_medications.any?
              accessible_medications.each do |person_medication|
                render Components::PersonMedications::Card.new(
                  person_medication: person_medication,
                  person: person,
                  todays_takes: takes_by_person_medication[person_medication.id],
                  current_user: current_user
                )
              end
            else
              render_my_medications_empty_state
            end
          end
        end
      end

      def render_empty_state
        div(class: 'col-span-full') do
          Card(class: 'text-center py-12 px-8 border-dashed border-2 bg-slate-50/50') do
            Text(size: '3', weight: 'medium', class: 'text-slate-400 mb-6') { t('people.show.no_schedules') }
            if can_create_schedule?
              Link(
                href: new_person_schedule_path(person),
                variant: :primary,
                class: 'rounded-xl',
                data: { turbo_stream: true }
              ) { t('people.show.add_first_schedule') }
            end
          end
        end
      end

      def render_my_medications_empty_state
        div(class: 'col-span-full') do
          Card(class: 'text-center py-12 px-8 border-dashed border-2 bg-slate-50/50') do
            div(class: 'space-y-2 mb-6') do
              Text(size: '3', weight: 'medium', class: 'text-slate-400') { t('people.show.no_medications') }
              Text(size: '2', class: 'text-slate-300') { t('people.show.medications_hint') }
            end
            if view_context.policy(PersonMedication.new(person: person)).create?
              Link(
                href: new_person_person_medication_path(person),
                variant: :primary,
                class: 'rounded-xl',
                data: { turbo_stream: true }
              ) { t('people.show.add_first_medication') }
            end
          end
        end
      end

      def can_create_schedule?
        view_context.policy(Schedule.new(person: person)).create?
      end

      def render_edit_form
        # Keep current logic for editing, just wrapping in new aesthetic container
        div(class: 'container mx-auto px-4 py-12 max-w-2xl') do
          Card(class: 'overflow-hidden border-none shadow-2xl') do
            form_with(model: person, class: 'space-y-8 p-10', data: { controller: 'auto-submit' }) do |f|
              div do
                Heading(level: 2, size: '6', class: 'font-bold mb-1') { t('people.form.edit_heading') }
                Text(size: '2', weight: 'muted') { t('people.form.edit_subheading', name: person.name) }
              end

              div(class: 'space-y-6') do
                div(class: 'space-y-2') do
                  render RubyUI::FormFieldLabel.new(
                    for: 'person_name',
                    class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 px-1'
                  ) { t('people.form.name') }
                  render f.text_field(
                    :name,
                    class: 'rounded-2xl border-slate-100 bg-slate-50/50 py-6 px-5 focus:bg-white focus:ring-4 ' \
                           'focus:ring-primary/5 focus:border-primary transition-all'
                  )
                end

                div(class: 'space-y-2') do
                  render RubyUI::FormFieldLabel.new(
                    for: 'person_date_of_birth',
                    class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 px-1'
                  ) { t('people.form.date_of_birth') }
                  render f.date_field(
                    :date_of_birth,
                    class: 'rounded-2xl border-slate-100 bg-slate-50/50 py-6 px-5 focus:bg-white focus:ring-4 ' \
                           'focus:ring-primary/5 focus:border-primary transition-all'
                  )
                end
              end

              div(class: 'flex gap-3 pt-4') do
                render Button.new(type: :submit, variant: :primary, class: 'flex-1 py-7 font-bold') {
                  t('people.form.save')
                }
                Link(href: person_path(person), variant: :ghost, class: 'py-7 px-8 font-bold text-slate-400') do
                  t('people.form.cancel')
                end
              end
            end
          end
        end
      end
    end
  end
end
