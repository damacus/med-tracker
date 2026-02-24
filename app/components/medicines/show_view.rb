# frozen_string_literal: true

module Components
  module Medicines
    class ShowView < Components::Base
      attr_reader :medicine, :notice

      def initialize(medicine:, notice: nil)
        @medicine = medicine
        @notice = notice
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-6xl space-y-12') do
          render_notice if notice.present?
          render_header

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-12') do
            div(class: 'lg:col-span-2 space-y-8') do
              render_description_section
              render_warnings_section if medicine.warnings.present?
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
                'Medicine Profile'
              end
              Heading(level: 1, size: '8', class: 'font-black tracking-tight') { medicine.name }
              div(class: 'flex items-center gap-1 mt-1') do
                render Icons::Home.new(size: 14, class: 'text-slate-400')
                Text(size: '2', class: 'text-slate-400') { medicine.location.name }
              end
            end
          end

          div(class: 'flex gap-3') do
            Link(href: edit_medicine_path(medicine), variant: :outline, size: :lg,
                 class: 'rounded-2xl font-bold text-sm bg-white') do
              'Edit Details'
            end
            Link(href: medicines_path, variant: :ghost, size: :lg,
                 class: 'rounded-2xl font-bold text-sm text-slate-400 hover:text-slate-600') do
              'Inventory'
            end
          end
        end
      end

      def render_description_section
        div(class: 'space-y-4') do
          Heading(level: 2, size: '5', class: 'font-bold tracking-tight') { 'Overview' }
          Card(class: 'p-8') do
            Text(size: '3', class: 'text-slate-600 leading-relaxed') do
              medicine.description.presence || 'No description provided.'
            end
          end
        end
      end

      def render_warnings_section
        div(class: 'space-y-4') do
          div(class: 'flex items-center gap-2') do
            render Icons::AlertCircle.new(size: 20, class: 'text-rose-500')
            Heading(level: 2, size: '5', class: 'font-bold tracking-tight text-rose-500') { 'Safety Warnings' }
          end
          Card(class: 'bg-rose-50 border-rose-100 p-8') do
            Text(size: '3', class: 'text-rose-800 leading-relaxed font-medium') { medicine.warnings }
          end
        end
      end

      def render_stock_card
        Card(class: 'p-8 space-y-6 overflow-hidden relative') do
          Heading(level: 3, size: '4', class: 'font-bold') { 'Inventory Status' }

          current = medicine.current_supply || 0
          threshold = [medicine.reorder_threshold, 1].max
          percentage = [current.to_f / threshold * 100, 100].min.round

          div(class: 'space-y-4') do
            div(class: 'flex items-baseline gap-2') do
              span(class: "text-5xl font-black #{medicine.low_stock? ? 'text-rose-600' : 'text-primary'}") do
                current.to_s
              end
              Text(size: '2', weight: 'bold', class: 'text-slate-400') { 'units remaining' }
            end

            div(class: 'space-y-2') do
              div(class: 'h-2 w-full bg-slate-50 rounded-full overflow-hidden') do
                div(class: "h-full #{medicine.low_stock? ? 'bg-rose-500' : 'bg-primary'} rounded-full",
                    style: "width: #{percentage}%")
              end
              div(
                class: 'flex justify-between items-center text-[10px] font-black uppercase ' \
                       'tracking-widest text-slate-400'
              ) do
                span { 'Supply Level' }
                span { "reorder at #{medicine.reorder_threshold}" }
              end
            end

            if medicine.low_stock?
              div(class: 'pt-2') do
                Badge(variant: :destructive, class: 'w-full py-2 rounded-xl justify-center text-xs tracking-wide') do
                  '⚠️ Low Stock Alert'
                end
              end
            end

            render_forecast_section
          end
        end
      end

      def render_forecast_section
        if medicine.forecast_available?
          div(class: 'pt-4 border-t border-slate-50 space-y-2') do
            if medicine.days_until_low_stock&.positive?
              forecast_item("Supply will be low in #{medicine.days_until_low_stock} days", :warning)
            end
            if medicine.days_until_out_of_stock&.positive?
              forecast_item("Supply will be empty in #{medicine.days_until_out_of_stock} days", :destructive)
            end
          end
        else
          div(class: 'pt-4 border-t border-slate-50') do
            Text(size: '2', class: 'text-slate-400 italic') { 'Forecast unavailable' }
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
          Heading(level: 3, size: '4', class: 'font-bold') { 'Standard Dosage' }

          if dosage_specified?
            div(class: 'flex items-center gap-4') do
              div(
                class: 'w-12 h-12 rounded-2xl bg-indigo-50 flex items-center justify-center text-indigo-600 shadow-sm'
              ) do
                render Icons::CheckCircle.new(size: 24)
              end
              div do
                span(class: 'text-3xl font-black text-slate-900') { medicine.dosage_amount.to_s }
                span(class: 'text-lg font-bold text-slate-400 ml-1') { medicine.dosage_unit }
              end
            end
          else
            Text(size: '2', class: 'text-slate-400 italic') { 'No standard dosage specified.' }
          end

          div(class: 'pt-4 border-t border-slate-50') do
            overview_item('Reorder At', "#{medicine.reorder_threshold} units", Icons::Settings)
          end
        end
      end

      def render_actions_card
        div(class: 'space-y-4') do
          render Button.new(variant: :primary, class: 'w-full py-7 rounded-2xl shadow-xl shadow-primary/20') {
            'Log Administration'
          }
          render Components::Medicines::RefillModal.new(
            medicine: medicine,
            options: {
              button_variant: :outline,
              button_class: 'w-full py-7 rounded-2xl bg-white'
            }
          )
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
        medicine.dosage_amount.present? && medicine.dosage_unit.present?
      end
    end
  end
end
