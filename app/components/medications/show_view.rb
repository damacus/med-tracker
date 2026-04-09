# frozen_string_literal: true

module Components
  module Medications
    class ShowView < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include Phlex::Rails::Helpers::TimeAgoInWords

      attr_reader :medication, :notice

      def initialize(medication:, notice: nil)
        @medication = medication
        @notice = notice
        super()
      end

      def view_template
        div(id: "medication_show_#{medication.id}", class: 'container mx-auto px-4 py-12 max-w-6xl space-y-12') do
          render_notice if notice.present?
          render_header

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-12') do
            div(class: 'lg:col-span-2 space-y-8') do
              render_description_section
              render_warnings_section if medication.warnings.present?
              render_dosages_section
            end

            div(class: 'space-y-8') do
              render_stock_card
              render_dosage_card
              render_actions_card
            end
          end
        end
      end

      private

      def render_notice
        render RubyUI::Alert.new(variant: :success, class: 'mb-8 rounded-2xl border-none shadow-sm') do
          plain(notice)
        end
      end

      def render_header
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b border-border') do
          div(class: 'flex items-center gap-6') do
            div(
              class: 'w-20 h-20 rounded-[2rem] bg-primary/10 flex items-center justify-center text-primary shadow-inner'
            ) do
              render Icons::Pill.new(size: 32)
            end
            div(class: 'space-y-1') do
              Text(size: '2', weight: 'black', class: 'uppercase tracking-[0.2em] font-bold opacity-40 block mb-1') do
                t('medications.show.profile')
              end
              Heading(level: 1, size: '8', class: 'font-black tracking-tight') { medication.name }
              div(class: 'flex items-center gap-1 mt-1') do
                render Icons::Home.new(size: 14, class: 'text-muted-foreground')
                Text(size: '2', class: 'text-muted-foreground') { medication.location.name }
              end
            end
          end

          div(class: 'flex gap-3') do
            Link(
              href: edit_medication_path(medication, return_to: medication_path(medication)),
              variant: :outline,
              size: :lg,
              class: 'rounded-2xl font-bold text-sm bg-surface-container-lowest'
            ) do
              render Icons::Pencil.new(size: 16, class: 'mr-2')
              plain t('medications.show.edit_details')
            end
            Link(
              href: medications_path,
              variant: :ghost,
              size: :lg,
              class: 'rounded-2xl font-bold text-sm text-muted-foreground hover:text-foreground'
            ) do
              t('medications.show.inventory')
            end
          end
        end
      end

      def render_description_section
        div(class: 'space-y-4') do
          Heading(level: 2, size: '5', class: 'font-bold tracking-tight') { t('medications.show.overview') }
          Card(class: 'p-8') do
            Text(size: '3', class: 'text-muted-foreground leading-relaxed') do
              medication.description.presence || t('medications.show.no_description')
            end
          end
        end
      end

      def render_warnings_section
        div(class: 'space-y-4') do
          div(class: 'flex items-center gap-2') do
            render Icons::AlertCircle.new(size: 20, class: 'text-on-error-container')
            Heading(level: 2, size: '5', class: 'font-bold tracking-tight text-on-error-container') do
              t('medications.show.safety_warnings')
            end
          end
          Card(class: 'bg-error-container border-error/20 p-8') do
            Text(size: '3', class: 'text-on-error-container leading-relaxed font-medium') { medication.warnings }
          end
        end
      end

      def render_stock_card
        Card(class: 'p-8 space-y-6 overflow-hidden relative') do
          Heading(level: 3, size: '4', class: 'font-bold') { t('medications.show.inventory_status') }

          current = medication.current_supply || 0
          percentage = medication.supply_percentage

          div(class: 'space-y-4') do
            div(class: 'flex items-baseline gap-2') do
              stock_count_class = if medication.low_stock?
                                    'text-5xl font-black text-on-error-container'
                                  else
                                    'text-5xl font-black text-primary'
                                  end

              span(class: stock_count_class) do
                current.to_s
              end
              Text(size: '2', weight: 'bold', class: 'text-muted-foreground') do
                current == 1 ? 'unit remaining' : 'units remaining'
              end
            end

            div(class: 'space-y-2') do
              div(class: 'h-2 w-full bg-surface-container-low rounded-full overflow-hidden') do
                div(class: "h-full #{medication.low_stock? ? 'bg-error' : 'bg-primary'} rounded-full",
                    style: "width: #{percentage}%")
              end
              div(
                class: 'flex justify-between items-center text-[10px] font-black uppercase ' \
                       'tracking-widest text-muted-foreground'
              ) do
                span { t('medications.show.supply_level') }
                span { t('medications.show.reorder_at', threshold: medication.reorder_threshold) }
              end
            end

            if medication.low_stock?
              div(class: 'pt-2 space-y-2') do
                Badge(variant: :destructive, class: 'w-full py-2 rounded-xl justify-center text-xs tracking-wide') do
                  t('medications.show.low_stock_alert')
                end

                render_reorder_status_badge if medication.reorder_status.present?
              end
            end

            render_forecast_section
          end
        end
      end

      def render_forecast_section
        if medication.forecast_available?
          div(class: 'pt-4 border-t border-surface-container-low space-y-2') do
            if medication.days_until_low_stock&.positive?
              forecast_item(t('medications.show.forecast.low_in_days', days: medication.days_until_low_stock), :warning)
            end
            if medication.days_until_out_of_stock&.positive?
              forecast_item(t('medications.show.forecast.empty_in_days', days: medication.days_until_out_of_stock),
                            :destructive)
            end
          end
        else
          div(class: 'pt-4 border-t border-surface-container-low') do
            Text(size: '2', class: 'text-muted-foreground italic') { t('medications.show.forecast_unavailable') }
          end
        end
      end

      def forecast_item(message, variant)
        text_class = variant == :destructive ? 'text-on-error-container' : 'text-on-warning-container'

        div(class: 'flex items-center gap-2') do
          render Icons::AlertCircle.new(size: 14, class: text_class)
          Text(size: '2', weight: 'medium', class: text_class) do
            message
          end
        end
      end

      def render_dosage_card
        Card(class: 'p-8 space-y-6') do
          Heading(level: 3, size: '4', class: 'font-bold') { t('medications.show.standard_dosage') }

          if dosage_specified?
            div(class: 'flex items-center gap-4') do
              div(
                class: 'w-12 h-12 rounded-2xl bg-secondary-container flex items-center justify-center ' \
                       'text-on-secondary-container shadow-sm'
              ) do
                render Icons::CheckCircle.new(size: 24)
              end
              div do
                span(class: 'text-3xl font-black text-foreground') { medication.dosage_amount.to_s }
                span(class: 'text-lg font-bold text-muted-foreground ml-1') { medication.dosage_unit }
              end
            end
          else
            Text(size: '2', class: 'text-muted-foreground italic') { t('medications.show.no_dosage') }
          end

          div(class: 'pt-4 border-t border-surface-container-low') do
            overview_item(t('medications.show.reorder_at_label'), pluralize(medication.reorder_threshold, 'unit'), Icons::Settings)
          end
        end
      end

      def render_actions_card
        div(class: 'space-y-3') do
          Link(
            href: add_medication_path(medication_id: medication.id),
            variant: :outline,
            size: :lg,
            class: 'w-full py-6 rounded-2xl bg-surface-container-lowest border-border shadow-sm ' \
                   'flex items-center justify-center'
          ) do
            render Icons::PlusCircle.new(size: 18, class: 'mr-2 text-primary')
            span(class: 'font-semibold text-foreground') { t('medications.show.add_schedule') }
          end

          render Button.new(
            variant: :primary,
            class: 'w-full py-6 rounded-2xl shadow-lg shadow-primary/20 flex items-center justify-center'
          ) do
            render Icons::Activity.new(size: 18, class: 'mr-2')
            span(class: 'font-semibold') { t('medications.show.log_administration') }
          end

          # Reorder & Refill Actions Group
          div(class: 'grid grid-cols-2 gap-3') do
            render_reorder_actions
            render_refill_modal
          end
        end
      end

      def render_reorder_actions
        path, label, icon = if medication.reorder_status.nil?
                              [mark_as_ordered_medication_path(medication), t('medications.show.mark_as_ordered'), Icons::Clock]
                            elsif medication.reorder_ordered?
                              [mark_as_received_medication_path(medication), t('medications.show.mark_as_received'), Icons::Check]
                            end

        return unless path

        Link(
          href: path,
          variant: :outline,
          size: :lg,
          data: { turbo_method: :patch },
          class: 'w-full py-6 rounded-2xl bg-surface-container-lowest border-border shadow-sm ' \
                 'flex items-center justify-center'
        ) do
          render icon.new(size: 18, class: 'mr-2 text-primary')
          span(class: 'font-semibold text-foreground') { label }
        end
      end

      def render_refill_modal
        is_received = medication.reorder_received?
        base_classes = 'w-full py-6 rounded-2xl shadow-sm flex items-center justify-center'
        button_class = if is_received
                         base_classes
                       else
                         "#{base_classes} bg-surface-container-lowest border-border"
                       end

        render Components::Medications::RefillModal.new(
          medication: medication,
          button_variant: is_received ? :primary : :outline,
          button_class: button_class,
          button_label: is_received ? t('medications.show.complete_refill') : t('medications.show.refill_inventory')
        )
      end

      def render_reorder_status_badge
        variant = case medication.reorder_status.to_sym
                  when :ordered then :default
                  when :received then :success
                  else :outline
                  end

        div(class: 'flex flex-col gap-1') do
          Badge(variant: variant, class: 'w-full py-2 rounded-xl justify-center text-xs tracking-wide') do
            t("medications.reorder_statuses.#{medication.reorder_status}")
          end

          timestamp = if medication.reorder_received?
                        medication.reordered_at
                      elsif medication.reorder_ordered?
                        medication.ordered_at
                      end

          if timestamp
            Text(size: '1', class: 'text-center text-muted-foreground font-medium') do
              status_text = t("medications.reorder_statuses.#{medication.reorder_status}")
              time_ago = time_ago_in_words(timestamp)
              "#{status_text} #{time_ago} ago"
            end
          end
        end
      end

      def overview_item(label, value, icon_class)
        div(class: 'flex items-center gap-4 group') do
          div(
            class: 'w-10 h-10 rounded-xl bg-surface-container-low flex items-center justify-center ' \
                   'text-muted-foreground ' \
                   'group-hover:bg-primary/5 group-hover:text-primary transition-colors'
          ) do
            render icon_class.new(size: 20)
          end
          div do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-muted-foreground') { label }
            Text(size: '2', weight: 'semibold') { value }
          end
        end
      end

      def render_dosages_section
        dosages = medication.dosages.sort_by(&:amount)
        can_manage = begin
          view_context.policy(medication).update?
        rescue StandardError
          false
        end

        Card(class: 'p-6') do
          div(class: 'flex items-center justify-between mb-4') do
            Heading(level: 3, size: '4', class: 'font-bold') { t('medications.show.dosages_heading') }
            if can_manage
              Link(
                href: view_context.new_medication_dosage_path(medication),
                variant: :outline,
                size: :sm,
                data: { turbo_frame: 'modal' }
              ) { t('medications.show.add_dosage') }
            end
          end

          turbo_frame_tag 'modal'

          if dosages.any?
            div(class: 'space-y-3') do
              dosages.each do |dosage|
                render_dosage_row(dosage, can_manage)
              end
            end
          else
            Text(size: '2', class: 'text-muted-foreground italic') { t('medications.show.no_dosages') }
          end
        end
      end

      def render_dosage_row(dosage, can_manage)
        div(class: 'flex items-start justify-between gap-3 rounded-lg border border-border p-3') do
          div(class: 'space-y-1') do
            render_dosage_summary(dosage)
            render_dosage_scheduling_hint(dosage)
          end

          if can_manage
            div(class: 'flex gap-2 flex-none') do
              Link(
                href: view_context.edit_medication_dosage_path(medication, dosage),
                variant: :ghost,
                size: :sm,
                data: { turbo_frame: 'modal' }
              ) { t('medications.show.edit_dosage') }
            end
          end
        end
      end

      def render_dosage_summary(dosage)
        div(class: 'flex items-center gap-2 flex-wrap') do
          span(class: 'font-semibold text-sm') { "#{dosage.amount.to_f} #{dosage.unit}" }
          span(class: 'text-muted-foreground text-sm') { dosage.frequency }
          if dosage.default_for_adults?
            Badge(variant: :outline, class: 'text-xs') { t('dosages.form.default_for_adults') }
          end
          if dosage.default_for_children?
            Badge(variant: :secondary, class: 'text-xs') { t('medications.show.children') }
          end
        end
      end

      def render_dosage_scheduling_hint(dosage)
        return unless dosage.default_max_daily_doses || dosage.default_min_hours_between_doses

        div(class: 'text-xs text-muted-foreground') do
          parts = []
          if dosage.default_max_daily_doses
            parts << t('medications.show.max_per_cycle', count: dosage.default_max_daily_doses)
          end
          if dosage.default_min_hours_between_doses
            parts << t('medications.show.min_hours_apart', hours: dosage.default_min_hours_between_doses)
          end
          plain parts.join(' · ')
        end
      end

      def dosage_specified?
        medication.dosage_amount.present? && medication.dosage_unit.present?
      end
    end
  end
end
