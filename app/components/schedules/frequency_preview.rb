# frozen_string_literal: true

module Components
  module Schedules
    class FrequencyPreview < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag

      FRAME_ID = 'schedule_frequency_preview'

      attr_reader :max_daily_doses, :min_hours_between_doses, :dose_cycle

      def initialize(max_daily_doses:, min_hours_between_doses:, dose_cycle:)
        @max_daily_doses = max_daily_doses
        @min_hours_between_doses = min_hours_between_doses
        @dose_cycle = dose_cycle
        super()
      end

      def view_template
        turbo_frame_tag(FRAME_ID, data: { schedule_form_target: 'frequencyPreview' }) do
          render_preview if phrase.present?
        end
      end

      private

      def render_preview
        div(
          class: 'rounded-shape-sm border border-outline-variant/40 bg-surface-container-low px-4 py-3',
          data: { testid: 'schedule-frequency-preview' }
        ) do
          m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') do
            span(class: 'font-bold text-foreground') { t('schedules.frequency_phrase.preview_label') }
            plain " #{phrase}"
          end
        end
      end

      def phrase
        @phrase ||= ScheduleFrequencyPhrase.new(
          max_daily_doses: max_daily_doses,
          min_hours_between_doses: min_hours_between_doses,
          dose_cycle: dose_cycle
        ).to_s
      end
    end
  end
end
