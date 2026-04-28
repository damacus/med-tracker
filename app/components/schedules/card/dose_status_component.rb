# frozen_string_literal: true

module Components
  module Schedules
    class Card
      class DoseStatusComponent < Components::Base
        attr_reader :schedule, :presenter

        def initialize(schedule:, presenter:)
          @schedule = schedule
          @presenter = presenter
          super()
        end

        def view_template
          CardContent(class: 'flex-grow space-y-6 px-8') do
            div(class: 'pt-4 border-t border-border space-y-4') do
              render_date_details
              render_notes if schedule.notes.present?
              render_countdown_notice if presenter.countdown_notice?
              render_takes_section
            end
          end
        end

        private

        def render_date_details
          div(class: 'flex items-center gap-6') do
            div do
              m3_text(variant: :label_small, class: 'uppercase tracking-widest text-on-surface-variant font-black') do
                t('schedules.card.started')
              end
              m3_text(variant: :body_medium, class: 'text-on-surface-variant font-bold') do
                schedule.start_date.strftime('%b %d, %Y')
              end
            end

            if schedule.end_date
              div do
                m3_text(variant: :label_small, class: 'uppercase tracking-widest text-on-surface-variant font-black') do
                  t('schedules.card.ends')
                end
                m3_text(variant: :body_medium, class: 'text-on-surface-variant font-bold') do
                  schedule.end_date.strftime('%b %d, %Y')
                end
              end
            end
          end
        end

        def render_notes
          div(class: 'p-4 bg-primary-container/40 border border-primary/20 rounded-2xl') do
            div(class: 'flex items-center gap-2 mb-1') do
              render Icons::AlertCircle.new(size: 14, class: 'text-on-primary-container')
              m3_text(variant: :label_small, class: 'uppercase tracking-widest text-on-primary-container font-black') do
                t('schedules.card.notes')
              end
            end
            m3_text(variant: :body_medium, class: 'text-on-primary-container leading-relaxed font-medium') do
              schedule.notes
            end
          end
        end

        def render_countdown_notice
          div(class: 'p-4 bg-error-container/20 border border-error/20 rounded-2xl') do
            div(class: 'flex items-center gap-2 mb-1') do
              render Icons::AlertCircle.new(size: 14, class: 'text-on-error-container')
              m3_text(variant: :label_small, class: 'uppercase tracking-widest text-on-error-container font-black') do
                t('schedules.card.next_dose_available')
              end
            end
            m3_text(variant: :body_medium, class: 'text-on-error-container font-bold') do
              schedule.countdown_display
            end
          end
        end

        def render_takes_section
          div(class: 'space-y-4 pt-2') do
            div(class: 'flex items-center justify-between') do
              m3_text(variant: :label_small, class: 'uppercase tracking-widest text-on-surface-variant font-black') do
                t('schedules.card.todays_doses')
              end
              if presenter.dose_count_badge?
                m3_badge(variant: :outlined, class: 'px-2 py-0.5 text-[10px] font-black') do
                  plain presenter.dose_count_label
                end
              end
            end
            render_todays_takes
          end
        end

        def render_todays_takes
          takes = presenter.resolved_todays_takes

          if takes.any?
            div(class: 'grid grid-cols-1 gap-2') do
              takes.each do |take|
                render_take_item(take)
              end
            end
          else
            m3_text(variant: :body_medium, class: 'italic text-on-surface-variant px-1 font-medium') do
              t('schedules.card.no_doses_today')
            end
          end
        end

        def render_take_item(take)
          div(
            class: 'flex items-center justify-between p-3 rounded-xl ' \
                   'bg-surface-container-low group/item transition-colors ' \
                   'hover:bg-surface-container-high'
          ) do
            div(class: 'flex items-center gap-3') do
              render Icons::CheckCircle.new(size: 16, class: 'text-primary')
              div(class: 'space-y-1') do
                m3_text(variant: :body_medium, class: 'text-foreground font-bold') do
                  take.taken_at.strftime('%l:%M %p').strip
                end
                if take.inventory_location.present?
                  m3_text(variant: :label_small, class: 'text-on-surface-variant font-medium') do
                    take.inventory_location.name
                  end
                end
              end
            end
            m3_text(variant: :label_small, class: 'text-on-surface-variant font-black uppercase tracking-widest') do
              "#{take.amount_ml.to_i}#{schedule.dose_unit}"
            end
          end
        end
      end
    end
  end
end
