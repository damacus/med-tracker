# frozen_string_literal: true

module Components
  module Medications
    class ShowView < Components::Base
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
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b border-slate-100') do
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
                render Icons::Home.new(size: 14, class: 'text-slate-400')
                Text(size: '2', class: 'text-slate-400') { medication.location.name }
              end
            end
          end

          div(class: 'flex gap-3') do
            Link(
              href: edit_medication_path(medication, return_to: medication_path(medication)),
              variant: :outline,
              size: :lg,
              class: 'rounded-2xl font-bold text-sm bg-white'
            ) do
              render Icons::Pencil.new(size: 16, class: 'mr-2')
              plain t('medications.show.edit_details')
            end
            Link(
              href: medications_path,
              variant: :ghost,
              size: :lg,
              class: 'rounded-2xl font-bold text-sm text-slate-400 hover:text-slate-600'
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
            Text(size: '3', class: 'text-slate-600 leading-relaxed') do
              medication.description.presence || t('medications.show.no_description')
            end
          end
        end
      end

      def render_warnings_section
        div(class: 'space-y-4') do
          div(class: 'flex items-center gap-2') do
            render Icons::AlertCircle.new(size: 20, class: 'text-rose-500')
            Heading(level: 2, size: '5', class: 'font-bold tracking-tight text-rose-500') do
              t('medications.show.safety_warnings')
            end
          end
          Card(class: 'bg-rose-50 border-rose-100 p-8') do
            Text(size: '3', class: 'text-rose-800 leading-relaxed font-medium') { medication.warnings }
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
              span(class: "text-5xl font-black #{medication.low_stock? ? 'text-rose-600' : 'text-primary'}") do
                current.to_s
              end
              Text(size: '2', weight: 'bold', class: 'text-slate-400') { t('medications.show.remaining') }
            end

            div(class: 'space-y-2') do
              div(class: 'h-2 w-full bg-slate-50 rounded-full overflow-hidden') do
                div(class: "h-full #{medication.low_stock? ? 'bg-rose-500' : 'bg-primary'} rounded-full",
                    style: "width: #{percentage}%")
              end
              div(
                class: 'flex justify-between items-center text-[10px] font-black uppercase ' \
                       'tracking-widest text-slate-400'
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
          div(class: 'pt-4 border-t border-slate-50 space-y-2') do
            if medication.days_until_low_stock&.positive?
              forecast_item(t('medications.show.forecast.low_in_days', days: medication.days_until_low_stock), :warning)
            end
            if medication.days_until_out_of_stock&.positive?
              forecast_item(t('medications.show.forecast.empty_in_days', days: medication.days_until_out_of_stock),
                            :destructive)
            end
          end
        else
          div(class: 'pt-4 border-t border-slate-50') do
            Text(size: '2', class: 'text-slate-400 italic') { t('medications.show.forecast_unavailable') }
          end
        end
      end

      def forecast_item(message, variant)
        div(class: 'flex items-center gap-2') do
          render Icons::AlertCircle.new(size: 14, class: variant == :destructive ? 'text-rose-500' : 'text-amber-500')
          Text(size: '2', weight: 'medium', class: variant == :destructive ? 'text-rose-700' : 'text-amber-700') do
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
                class: 'w-12 h-12 rounded-2xl bg-indigo-50 flex items-center justify-center text-indigo-600 shadow-sm'
              ) do
                render Icons::CheckCircle.new(size: 24)
              end
              div do
                span(class: 'text-3xl font-black text-slate-900') { medication.dosage_amount.to_s }
                span(class: 'text-lg font-bold text-slate-400 ml-1') { medication.dosage_unit }
              end
            end
          else
            Text(size: '2', class: 'text-slate-400 italic') { t('medications.show.no_dosage') }
          end

          div(class: 'pt-4 border-t border-slate-50') do
            overview_item(t('medications.show.reorder_at_label'), "#{medication.reorder_threshold} units", Icons::Settings)
          end
        end
      end

      def render_actions_card
        div(class: 'space-y-4') do
          render Button.new(variant: :primary, class: 'w-full py-7 rounded-2xl shadow-xl shadow-primary/20') {
            t('medications.show.log_administration')
          }

          render_reorder_actions if medication.low_stock?
          render_refill_modal
        end
      end

      def render_reorder_actions
        if medication.reorder_status.nil?
          render_reorder_link(mark_as_ordered_medication_path(medication), 'Mark as Ordered', Icons::Clock)
        elsif medication.reorder_ordered?
          render_reorder_link(mark_as_received_medication_path(medication), 'Mark as Received', Icons::Check)
        end
      end

      def render_reorder_link(path, label, icon_class)
        Link(href: path, variant: :outline, size: :lg,
             data: { turbo_method: :patch },
             class: 'w-full py-7 rounded-2xl bg-white flex items-center justify-center') do
          render icon_class.new(size: 18, class: 'mr-2')
          plain label
        end
      end

      def render_refill_modal
        button_class = if medication.reorder_received?
                         'w-full py-7 rounded-2xl'
                       else
                         'w-full py-7 rounded-2xl bg-white'
                       end

        render Components::Medications::RefillModal.new(
          medication: medication,
          button_variant: medication.reorder_received? ? :primary : :outline,
          button_class: button_class,
          button_label: medication.reorder_received? ? 'Complete Refill' : nil
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
            Text(size: '1', class: 'text-center text-slate-400 font-medium') do
              status_text = t("medications.reorder_statuses.#{medication.reorder_status}")
              time_ago = helpers.time_ago_in_words(timestamp)
              "#{status_text} #{time_ago} ago"
            end
          end
        end
      end

      def overview_item(label, value, icon_class)
        div(class: 'flex items-center gap-4 group') do
          div(
            class: 'w-10 h-10 rounded-xl bg-slate-50 flex items-center justify-center text-slate-400 ' \
                   'group-hover:bg-primary/5 group-hover:text-primary transition-colors'
          ) do
            render icon_class.new(size: 20)
          end
          div do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-400') { label }
            Text(size: '2', weight: 'semibold') { value }
          end
        end
      end

      def dosage_specified?
        medication.dosage_amount.present? && medication.dosage_unit.present?
      end
    end
  end
end
