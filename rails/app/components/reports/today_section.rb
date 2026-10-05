# frozen_string_literal: true

module Components
  module Reports
    class TodaySection < Components::Base
      def initialize(today_taken_medications:)
        @today_taken_medications = today_taken_medications
        super()
      end

      def view_template
        section(id: 'today', class: 'space-y-5 scroll-mt-24') do
          div(class: 'space-y-2') do
            m3_heading(level: 2, size: '5', class: 'font-bold') { translate('title') }
            m3_text(size: '2', class: 'text-on-surface-variant') { translate('description') }
          end

          if @today_taken_medications.any?
            div(class: 'grid grid-cols-1 gap-4') do
              @today_taken_medications.each { |group| render_group(group) }
            end
          else
            render_empty_state
          end
        end
      end

      private

      def render_group(group)
        m3_card(class: 'border border-border/70 bg-card p-5 shadow-elevation-1') do
          div(class: 'flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between') do
            div(class: 'flex items-center gap-3') do
              render Components::Shared::PersonAvatar.new(person: group.person, size: :sm)
              m3_heading(level: 3, size: '3', class: 'font-bold') { group.person.name }
            end
            div(class: 'flex flex-wrap gap-2') do
              group.medications.each do |medication|
                m3_badge(variant: :secondary, class: 'rounded-full px-3 py-1 font-semibold') { medication.name }
              end
            end
          end
        end
      end

      def render_empty_state
        m3_card(class: 'border border-dashed border-border bg-card p-6 text-center') do
          m3_text(size: '2', class: 'font-semibold text-on-surface-variant') { translate('empty') }
        end
      end

      def translate(key)
        I18n.t("reports.index.today.#{key}")
      end
    end
  end
end
