# frozen_string_literal: true

module Components
  module Medications
    class AdministrationModal < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :medication, :schedules, :person_medications, :current_user

      def initialize(medication:, schedules:, person_medications:, current_user:)
        @medication = medication
        @schedules = schedules
        @person_medications = person_medications
        @current_user = current_user
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          Dialog(open: true) do
            DialogContent(
              size: :xl,
              class: 'overflow-hidden border-border/50 bg-white shadow-[0_32px_90px_rgba(15,23,42,0.18)]'
            ) do
              DialogHeader(class: 'bg-gradient-to-b from-[#eefbf4] to-white px-8 pt-8 pb-4') do
                DialogTitle { t('medications.administration.title', medication: medication.name) }
                DialogDescription { t('medications.administration.subtitle') }
              end
              DialogMiddle(class: 'bg-[#fcfffd] px-8 pb-8 pt-4') do
                if administration_options.empty?
                  render_empty_state
                else
                  div(class: 'grid gap-4') do
                    administration_options.each do |option|
                      render_option(option)
                    end
                  end
                end
              end
            end
          end
        end
      end

      private

      def administration_options
        @administration_options ||= (schedules + person_medications).sort_by do |option|
          [option.person.name, option.class.name, option.id]
        end
      end

      def render_option(option)
        div(class: 'rounded-[28px] border border-border/60 bg-white/90 p-5 shadow-sm') do
          div(class: 'flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between') do
            div(class: 'space-y-2') do
              Text(size: '1', weight: 'black', class: 'uppercase tracking-[0.18em] text-muted-foreground') do
                option_label(option)
              end
              Heading(level: 2, size: '4', class: 'font-semibold tracking-tight') { option.person.name }
              Text(size: '2', class: 'text-muted-foreground') { option_summary(option) }
            end

            render Components::Medications::TakeAction.new(
              source: option,
              context: { person: option.person, current_user: current_user },
              amount: option.default_dose_amount,
              button: {
                label: t('medications.show.log_administration'),
                variant: :primary,
                size: :lg,
                class: 'w-full rounded-full lg:w-auto',
                testid: "log-administration-#{option.class.name.underscore}-#{option.id}",
                form_class: 'w-full lg:w-auto'
              }
            )
          end
        end
      end

      def option_label(option)
        option.is_a?(Schedule) ? t('medications.administration.scheduled') : t('medications.administration.as_needed')
      end

      def option_summary(option)
        [option.medication.name, option.dose_display, option.frequency.presence].compact.join(' • ')
      end

      def render_empty_state
        div(class: 'rounded-[28px] border border-dashed border-border/70 bg-[#fcfffd] px-6 py-10 text-center') do
          Heading(level: 2, size: '4', class: 'font-semibold tracking-tight') do
            t('medications.administration.empty_title')
          end
          Text(size: '2', class: 'mt-2 text-muted-foreground') do
            t('medications.administration.empty_description')
          end
        end
      end
    end
  end
end
