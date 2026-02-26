# frozen_string_literal: true

module Components
  module Dashboard
    class TimelineItem < Components::Base
      attr_reader :dose, :current_user

      def initialize(dose:, current_user: nil)
        @dose = dose
        @current_user = current_user
        super()
      end

      def view_template
        card_id = "timeline_#{dose[:source].class.name.underscore}_#{dose[:source].id}"
        render RubyUI::Card.new(
          class: "border-none border-l-4 #{status_border_class} transition-all duration-300 " \
                 'hover:scale-[1.01] hover:shadow-md bg-white shadow-sm',
          id: card_id,
          data: { id: "dose_#{dose_id}" }
        ) do
          div(class: 'flex items-center justify-between p-4') do
            div(class: 'flex items-center gap-4') do
              div(class: 'text-sm font-bold text-slate-500 w-12 hidden md:block') do
                if dose[:scheduled_at]
                  dose[:scheduled_at].strftime('%H:%M')
                else
                  '--:--'
                end
              end
              div do
                Heading(level: 3, size: '4', class: 'font-semibold') { dose[:source].medication.name }
                Text(size: '2', weight: 'muted') { subtitle_text }
              end
            end

            div(class: 'flex items-center gap-3') do
              render_action_button if dose[:status] == :upcoming
              status_badge
            end
          end
        end
      end

      private

      def own_dose?
        return true if current_user.nil?

        current_user.person == dose[:person]
      end

      def take_label
        own_dose? ? t('person_medications.card.take') : t('person_medications.card.give')
      end

      def status_border_class
        case dose[:status]
        when :taken then 'border-l-emerald-500'
        when :upcoming then 'border-l-blue-500'
        when :cooldown then 'border-l-amber-500'
        when :out_of_stock then 'border-l-rose-500'
        else 'border-l-slate-200'
        end
      end

      def dose_id
        "#{dose[:source].class.name.downcase}_#{dose[:source].id}"
      end

      def subtitle_text
        person_name = dose[:person].name

        if dose[:status] == :taken && dose[:taken_at]
          time = dose[:taken_at].strftime('%l:%M %p').strip
          t('dashboard.dose_taken_at', person: person_name, time: time)
        else
          person_name
        end
      end

      def render_action_button
        source = dose[:source]
        is_schedule = source.is_a?(::Schedule)

        path = if is_schedule
                 take_medication_person_schedule_path(dose[:person], source)
               else
                 take_medication_person_person_medication_path(dose[:person], source)
               end
        amount = is_schedule ? source.dosage.amount : source.medication.dosage_amount

        form_with(url: path, method: :post,
                  data: { controller: 'optimistic-take', action: 'submit->optimistic-take#submit' }) do |f|
          input(type: :hidden, name: 'authenticity_token', value: view_context.form_authenticity_token)
          f.hidden_field :amount_ml, value: amount
          Button(
            type: :submit,
            variant: :outline,
            size: :md,
            data: { optimistic_take_target: 'button', testid: "take-dose-#{dose_id}" }
          ) { take_label }
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
