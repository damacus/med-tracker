# frozen_string_literal: true

module Components
  module Medications
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Pluralize

      attr_reader :medication, :title, :subtitle, :locations, :return_to

      def initialize(medication:, title:, subtitle: nil, locations: [], return_to: nil)
        @medication = medication
        @title = title
        @subtitle = subtitle
        @locations = locations
        @return_to = return_to
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-2xl') do
          render_header
          render_form
        end
      end

      private

      def render_header
        div(class: 'text-center mb-6 space-y-1') do
          m3_text(variant: :label_medium, class: 'uppercase tracking-[0.2em] font-black opacity-40') do
            t('forms.medications.inventory_management')
          end
          m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight text-foreground') { title }
          m3_text(variant: :body_large, class: 'text-on-surface-variant') { subtitle } if subtitle
        end
      end

      def render_form
        form_with(
          model: medication,
          class: 'space-y-8',
          data: { testid: 'medication-form' }
        ) do |form|
          render_errors(form) if medication.errors.any?
          render_hidden_form_state
          render_form_card(form)
        end
      end

      def render_hidden_form_state
        render_hidden_input('return_to', return_to) if return_to.present?
        render_hidden_input('medication[barcode]', medication.barcode) if medication.barcode.present?
        render_hidden_input('medication[default_schedule_type]', medication.default_schedule_type || 'multiple_daily')
        render_hidden_input('medication[default_schedule_config]', medication.default_schedule_config.to_json)
        render_hidden_dmd_state
      end

      def render_hidden_dmd_state
        return if medication.dmd_code.blank?

        render_hidden_input('medication[dmd_code]', medication.dmd_code)
        render_hidden_input('medication[dmd_system]', medication.dmd_system)
        render_hidden_input('medication[dmd_concept_class]', medication.dmd_concept_class)
      end

      def render_hidden_input(name, value)
        input(type: 'hidden', name: name, value: value)
      end

      def render_form_card(_form)
        m3_card(variant: :elevated, class: 'overflow-visible border-none shadow-elevation-3 rounded-[2.5rem]') do
          div(class: 'p-10 space-y-8') do
            div(class: 'space-y-6') do
              render Components::Medications::IdentityFields.new(
                medication: medication,
                locations: locations
              )
            end

            div(class: 'h-px bg-outline-variant w-full opacity-50')

            div(class: 'space-y-6') do
              m3_heading(variant: :title_large, level: 3, class: 'font-bold text-foreground') do
                t('forms.medications.supply', default: 'Supply')
              end
              render Components::Medications::SupplyFields.new(medication: medication)
            end

            div(class: 'h-px bg-outline-variant w-full opacity-50')

            render Components::Medications::DosageOptionsFields.new(medication: medication)

            div(class: 'h-px bg-outline-variant w-full opacity-50')

            render Components::Medications::WarningsField.new(medication: medication)
          end

          div(
            class: 'px-10 py-6 bg-surface-container-low border-t border-outline-variant/30 ' \
                   'flex items-center justify-between gap-4 rounded-b-[2.5rem]'
          ) do
            m3_link(href: return_to.presence || medications_path, variant: :text, size: :lg,
                    class: 'font-bold text-on-surface-variant hover:text-foreground') do
              t('forms.medications.back')
            end
            m3_button(type: :submit, variant: :filled, size: :lg,
                      class: 'px-8 rounded-shape-xl shadow-lg shadow-primary/20') do
              t('forms.medications.save_medication')
            end
          end
        end
      end

      def render_errors(_form)
        render RubyUI::Alert.new(variant: :destructive,
                                 class: 'mb-8 rounded-shape-xl border-none shadow-elevation-1') do
          div(class: 'flex items-start gap-3') do
            render Icons::AlertCircle.new(size: 20)
            div do
              m3_heading(variant: :title_medium, level: 2, class: 'font-bold mb-1') do
                plain t('forms.medications.validation_errors', count: medication.errors.count)
              end
              ul(class: 'text-sm opacity-90 list-disc pl-4 space-y-1 font-medium') do
                medication.errors.full_messages.each do |message|
                  li { message }
                end
              end
            end
          end
        end
      end
    end
  end
end
