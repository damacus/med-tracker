# frozen_string_literal: true

module Components
  module Dashboard
    class TimelineItem < Components::Base
      attr_reader :dose

      def initialize(dose:)
        @dose = dose
        super()
      end

      def view_template
        render RubyUI::Card.new(class: 'mb-4', data: { id: "dose_#{dose_id}" }) do
          div(class: 'flex items-center justify-between p-4') do
            div(class: 'flex items-center space-x-4') do
              status_icon

              div do
                render RubyUI::Heading.new(level: 4) { dose[:source].medicine.name }
                render RubyUI::Text.new(size: '2', weight: 'muted') do
                  "#{dose[:person].name} • #{dose[:scheduled_at].strftime('%H:%M')}"
                end
              end
            end

            div(class: 'flex items-center space-x-2') do
              render_action_button if dose[:status] == :upcoming
              status_badge
            end
          end
        end
      end

      private

      def dose_id
        "#{dose[:source].class.name.downcase}_#{dose[:source].id}"
      end

      def render_action_button
        path = if dose[:source].is_a?(Prescription)
                 take_medicine_person_prescription_path(dose[:person], dose[:source])
               else
                 take_medicine_person_person_medicine_path(dose[:person], dose[:source])
               end

        amount = if dose[:source].is_a?(Prescription)
                   dose[:source].dosage.amount
                 else
                   dose[:source].medicine.dosage_amount
                 end

        form_with(url: path, method: :post,
                  data: { controller: 'optimistic-take', action: 'submit->optimistic-take#submit' }) do |f|
          input(type: :hidden, name: 'authenticity_token', value: view_context.form_authenticity_token)
          f.hidden_field :amount_ml, value: amount
          button_classes = [
            'inline-flex items-center justify-center rounded-md text-sm font-medium',
            'ring-offset-background transition-colors focus-visible:outline-none',
            'focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
            'disabled:pointer-events-none disabled:opacity-50 border border-input',
            'bg-background hover:bg-accent hover:text-accent-foreground h-9 px-3'
          ].join(' ')
          f.button(type: :submit, class: button_classes, data: { optimistic_take_target: 'button' }) do
            t('person_medicines.card.take')
          end
        end
      end

      def status_icon
        case dose[:status]
        when :taken
          render Icons::CheckCircle.new(class: 'text-green-500 h-6 w-6')
        when :upcoming
          render Icons::Pill.new(class: 'text-blue-500 h-6 w-6')
        when :cooldown
          render Icons::AlertCircle.new(class: 'text-amber-500 h-6 w-6')
        when :out_of_stock
          render Icons::XCircle.new(class: 'text-red-500 h-6 w-6')
        else
          render Icons::AlertCircle.new(class: 'text-red-500 h-6 w-6')
        end
      end

      def status_badge
        variant_map = {
          taken: :success,
          upcoming: :default,
          cooldown: :warning,
          out_of_stock: :destructive
        }
        variant = variant_map[dose[:status]] || :destructive

        label = if dose[:status] == :cooldown && dose[:source].respond_to?(:countdown_display)
                  "#{t('dashboard.statuses.cooldown')} (#{dose[:source].countdown_display})"
                else
                  t("dashboard.statuses.#{dose[:status]}")
                end

        render RubyUI::Badge.new(variant: variant) do
          label
        end
      end
    end
  end
end
