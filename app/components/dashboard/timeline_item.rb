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
        render M3::Card.new(
          variant: :elevated,
          class: "border-none border-l-4 #{status_border_class} transition-all duration-300 " \
                 'hover:scale-[1.01] hover:shadow-elevation-2 bg-surface-container-low',
          id: card_id,
          data: { id: "dose_#{dose_id}" }
        ) do
          div(class: 'flex items-center justify-between p-5') do
            div(class: 'flex items-center gap-5') do
              m3_text(variant: :label_large, class: 'text-on-surface-variant w-14 hidden md:block font-black') do
                if dose[:scheduled_at]
                  dose[:scheduled_at].strftime('%H:%M')
                else
                  '--:--'
                end
              end
              div do
                m3_heading(variant: :title_medium, level: 3, class: 'font-bold tracking-tight') do
                  dose[:source].medication.name
                end
                m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') { subtitle_text }
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
        when :taken then 'border-l-success'
        when :upcoming then 'border-l-primary'
        when :cooldown then 'border-l-warning'
        when :out_of_stock then 'border-l-error'
        else 'border-l-border'
        end
      end

      def dose_id
        "#{dose[:source].class.name.downcase}_#{dose[:source].id}"
      end

      def subtitle_text
        person_name = dose[:person].name

        if dose[:status] == :taken && dose[:taken_at]
          time = dose[:taken_at].strftime('%l:%M %p').strip
          location_name = dose[:taken_from_location_name]
          if location_name.present?
            "#{t('dashboard.dose_taken_at', person: person_name, time: time)} • #{location_name}"
          else
            t('dashboard.dose_taken_at', person: person_name, time: time)
          end
        else
          person_name
        end
      end

      def render_action_button
        source = dose[:source]
        amount = source.dose_amount

        render Components::Medications::TakeAction.new(
          source: source,
          context: { person: dose[:person], current_user: current_user },
          amount: amount,
          button: {
            label: take_label,
            variant: :outlined,
            size: :md,
            testid: "take-dose-#{dose_id}",
            form_class: nil
          }
        )
      end

      def status_icon
        case dose[:status]
        when :taken
          render Icons::CheckCircle.new(size: 24, class: 'text-on-success-container')
        when :upcoming
          render Icons::Pill.new(size: 24, class: 'text-on-primary-container')
        when :cooldown
          render Icons::AlertCircle.new(size: 24, class: 'text-on-warning-container')
        when :out_of_stock
          render Icons::XCircle.new(size: 24, class: 'text-on-error-container')
        else
          render Icons::AlertCircle.new(size: 24, class: 'text-on-error-container')
        end
      end

      def status_badge
        variant_map = {
          taken: :tonal,
          upcoming: :filled,
          cooldown: :outlined,
          out_of_stock: :destructive
        }
        m3_variant = variant_map[dose[:status]] || :destructive

        label = if dose[:status] == :cooldown && dose[:source].respond_to?(:countdown_display)
                  "#{t('dashboard.statuses.cooldown')} (#{dose[:source].countdown_display})"
                else
                  t("dashboard.statuses.#{dose[:status]}")
                end

        m3_badge(variant: m3_variant, class: 'px-3 py-1 text-[10px] font-black uppercase tracking-wider') do
          label
        end
      end
    end
  end
end
