# frozen_string_literal: true

module Components
  module Dashboard
    class TimelineItem < Components::Base
      ACCENT_PALETTES = %w[blue violet teal amber].freeze

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
          class: "overflow-hidden rounded-shape-xl border #{card_chrome_class} bg-card shadow-elevation-2",
          id: card_id,
          data: {
            id: "dose_#{dose_id}",
            testid: "dashboard-medicine-card-#{dose_id}",
            status: dose[:status],
            palette: palette_name
          }
        ) do
          div(class: 'p-4') do
            div(class: 'flex items-start gap-4') do
              render_icon_well

              div(class: 'min-w-0 flex-1') do
                div(class: 'flex items-start justify-between gap-3') do
                  m3_heading(
                    variant: :title_medium,
                    level: 3,
                    class: 'min-w-0 font-black leading-tight tracking-tight'
                  ) do
                    dose[:source].medication.name
                  end
                  status_badge
                end

                m3_text(
                  variant: :body_medium,
                  class: 'mt-2 flex flex-wrap items-center gap-x-2 gap-y-1 text-on-surface-variant'
                ) do
                  plain metadata_text
                end

                div(class: 'mt-4 flex justify-end') do
                  render_action_button if actionable_status?
                end
              end
            end
          end
        end
      end

      private

      def render_icon_well
        div(
          class: "flex h-16 w-16 shrink-0 items-center justify-center rounded-shape-xl #{icon_well_class}",
          data: { testid: 'dashboard-medicine-icon', palette: palette_name }
        ) do
          render status_icon
        end
      end

      def own_dose?
        return true if current_user.nil?

        current_user.person == dose[:person]
      end

      def take_label
        own_dose? ? t('person_medications.card.take') : t('person_medications.card.give')
      end

      def card_chrome_class
        "dashboard-dose-card dashboard-dose-card--#{palette_name}"
      end

      def icon_well_class
        "dashboard-dose-icon dashboard-dose-icon--#{palette_name}"
      end

      def action_button_class
        'min-w-32 rounded-shape-full px-6 font-black text-base shadow-elevation-2 ' \
          "dashboard-dose-action dashboard-dose-action--#{palette_name}"
      end

      def palette_name
        return 'green' if dose[:status] == :taken
        return 'amber' if %i[cooldown max_reached].include?(dose[:status])
        return 'rose' if dose[:status] == :out_of_stock
        return 'teal' unless own_dose?

        ACCENT_PALETTES[dose[:source].id.to_i % ACCENT_PALETTES.size]
      end

      def dose_id
        "#{dose[:source].class.name.downcase}_#{dose[:source].id}"
      end

      def metadata_text
        [dose[:person].name, dose_time_text, location_name].compact_blank.join(' · ')
      end

      def dose_time_text
        time = dose[:taken_at] || dose[:scheduled_at]
        time&.strftime('%l:%M %p')&.strip
      end

      def location_name
        dose[:taken_from_location_name].presence || dose[:source].medication.location&.name
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
            variant: :filled,
            size: :md,
            icon: Icons::Pill,
            class: action_button_class,
            testid: "take-dose-#{dose_id}",
            form_class: 'w-full sm:w-auto'
          }
        )
      end

      def actionable_status?
        %i[upcoming available].include?(dose[:status])
      end

      def status_icon
        case dose[:status]
        when :taken
          Icons::CheckCircle.new(size: 34)
        when :cooldown, :max_reached
          Icons::Clock.new(size: 34)
        when :out_of_stock
          Icons::XCircle.new(size: 34)
        else
          Icons::Pill.new(size: 34)
        end
      end

      def status_badge
        m3_badge(
          variant: :tonal,
          class: "shrink-0 px-3 py-1 text-xs font-black dashboard-dose-status #{status_badge_class}"
        ) do
          status_label
        end
      end

      def status_badge_class
        "dashboard-dose-status--#{palette_name}"
      end

      def status_label
        case dose[:status]
        when :available
          t('dashboard.statuses.available_now')
        when :max_reached
          t('dashboard.statuses.max_reached')
        when :cooldown
          cooldown_label
        else
          t("dashboard.statuses.#{dose[:status]}")
        end
      end

      def cooldown_label
        if dose[:source].respond_to?(:countdown_display)
          "#{t('dashboard.statuses.cooldown')} (#{dose[:source].countdown_display})"
        else
          t('dashboard.statuses.cooldown')
        end
      end
    end
  end
end
