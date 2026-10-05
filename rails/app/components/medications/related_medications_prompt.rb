# frozen_string_literal: true

module Components
  module Medications
    class RelatedMedicationsPrompt < Components::Base
      def initialize(medications:, heading_id:)
        @medications = medications
        @heading_id = heading_id
      end

      def view_template
        aside(
          class: 'mt-3 rounded-lg border border-secondary/50 bg-secondary-container/20 p-3',
          aria: { labelledby: heading_id },
          data: { testid: 'related-medications-prompt' }
        ) do
          CardTitle(id: heading_id, class: 'text-xs text-on-secondary-container') do
            I18n.t('medications.finder.related_medications_title')
          end
          CardDescription(class: 'mt-1 text-xs text-on-secondary-container') do
            I18n.t('medications.finder.related_medications_message')
          end
          ul(class: 'mt-2 space-y-2') { medications.each { |medication| medication_item(medication) } }
        end
      end

      private

      attr_reader :heading_id, :medications

      def medication_item(medication)
        li(class: 'text-xs text-on-surface-variant') do
          a(
            href: medication.fetch(:path),
            class: 'font-bold text-primary underline underline-offset-2'
          ) { medication.fetch(:name) }
          medication_details(medication)
        end
      end

      def medication_details(medication)
        dl(class: 'mt-1 grid grid-cols-[auto_1fr] gap-x-2') do
          detail(
            I18n.t('medications.finder.related_medications_location'),
            medication.fetch(:location)
          )
          detail(
            I18n.t('medications.finder.related_medications_current_supply'),
            medication.fetch(:current_supply)
          )
        end
      end

      def detail(label, value)
        dt(class: 'font-bold') { label }
        dd { value }
      end
    end
  end
end
