# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a person's medication schedule within the dashboard
    class PersonSchedule < Components::Base
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :person, :prescriptions, :take_medicine_url_generator

      def initialize(person:, prescriptions:, take_medicine_url_generator: nil)
        @person = person
        @prescriptions = prescriptions
        @take_medicine_url_generator = take_medicine_url_generator
        super()
      end

      def view_template
        div(class: 'schedule-person') do
          render_person_header
          render_prescriptions_list
        end
      end

      private

      def render_person_header
        div(class: 'schedule-person__header') do
          h3(class: 'schedule-person__name') { person.name }
          p(class: 'schedule-person__age') { "Age: #{person.age}" }
        end
      end

      def render_prescriptions_list
        div(class: 'schedule-prescriptions') do
          prescriptions.each do |prescription|
            render_prescription_card(prescription)
          end
        end
      end

      def render_prescription_card(prescription)
        div(id: "prescription_#{prescription.id}", class: 'prescription-card') do
          div(class: 'prescription-card__content') do
            render_prescription_info(prescription)
            render_prescription_actions(prescription)
          end
        end
      end

      def render_prescription_info(prescription)
        div(class: 'prescription-card__info') do
          render_medicine_name(prescription)
          render_dosage_detail(prescription)
          render_frequency_detail(prescription)
          render_end_date_detail(prescription)
        end
      end

      def render_medicine_name(prescription)
        h4(class: 'prescription-card__medicine') { prescription.medicine.name }
      end

      def render_dosage_detail(prescription)
        amount = prescription.dosage&.amount
        unit = prescription.dosage&.unit
        dosage = [amount, unit].compact.join(' ')

        p(class: 'prescription-card__detail') { "Dosage: #{dosage}" }
      end

      def render_frequency_detail(prescription)
        return if prescription.frequency.blank?

        p(class: 'prescription-card__detail') { "Frequency: #{prescription.frequency}" }
      end

      def render_end_date_detail(prescription)
        return unless prescription.end_date

        formatted_date = prescription.end_date.strftime('%B %d, %Y')
        p(class: 'prescription-card__detail') { "Ends: #{formatted_date}" }
      end

      def render_prescription_actions(prescription)
        div(class: 'prescription-card__actions') do
          render_take_medicine_button(prescription)
        end
      end

      def render_take_medicine_button(prescription)
        if take_medicine_url_generator
          button_to_take_medicine(prescription)
        else
          button(class: 'quick-action__button', data: { test_id: "take-medicine-#{prescription.id}" }) do
            'Take Now'
          end
        end
      end

      def button_to_take_medicine(prescription)
        url = take_medicine_url_generator.call(prescription)
        button_to(
          url,
          method: :post,
          class: 'quick-action__button',
          data: { test_id: "take-medicine-#{prescription.id}" }
        ) do
          'Take Now'
        end
      end
    end
  end
end
