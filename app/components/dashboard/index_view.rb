# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      attr_reader :presenter

      def initialize(presenter:)
        @presenter = presenter
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl', data: { testid: 'dashboard' }) do
          render_header
          render_stats_section

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-8') do
            div(class: 'lg:col-span-2 space-y-8') do
              render_timeline_section
              render_health_insights
            end
            div(class: 'space-y-8') do
              render_prescriptions_section
              render_supply_levels
            end
          end
        end
      end

      private

      delegate :people, :active_prescriptions, :upcoming_prescriptions,
               :current_user, :doses, to: :presenter

      def render_header
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
          div do
            Text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
              Time.current.strftime('%A, %b %d')
            end
            Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
              current_user.person&.name ? "Good morning, #{current_user.person.name.split.first}" : t('dashboard.title')
            end
          end
          div(class: 'flex gap-3') do
            render RubyUI::Link.new(
              href: new_person_path,
              variant: :outline,
              size: :lg
            ) do
              t('dashboard.quick_actions.add_person')
            end
            render RubyUI::Link.new(
              href: new_medicine_path,
              variant: :primary,
              size: :lg
            ) do
              render Icons::Pill.new(size: 20, class: 'mr-2')
              span { t('dashboard.quick_actions.add_medicine') }
            end
          end
        end
      end

      def render_stats_section
        div(class: 'grid grid-cols-2 lg:grid-cols-4 gap-4 mb-12') do
          render Components::Dashboard::StatCard.new(
            title: t('dashboard.stats.people'),
            value: people.count,
            icon_type: 'users'
          )
          render Components::Dashboard::StatCard.new(
            title: t('dashboard.stats.active_prescriptions'),
            value: active_prescriptions.count,
            icon_type: 'pill'
          )
          # Placeholder stats to match mockup visual density
          render Components::Dashboard::StatCard.new(
            title: 'Compliance',
            value: '94%',
            icon_type: 'check'
          )
          render Components::Dashboard::StatCard.new(
            title: 'Next Dose',
            value: '12:30',
            icon_type: 'clock'
          )
        end
      end

      def render_timeline_section
        div(class: 'space-y-6') do
          div(class: 'flex items-center justify-between mb-2') do
            Heading(level: 2, size: '5', class: 'font-bold') { t('dashboard.todays_schedule') }
          end

          if doses.any?
            div(class: 'space-y-4') do
              doses.each do |dose|
                render Components::Dashboard::TimelineItem.new(dose: dose, current_user: current_user)
              end
            end
          else
            render RubyUI::Card.new(class: 'p-12 text-center rounded-[2rem] border-dashed border-2') do
              Text(weight: :muted) { t('dashboard.empty_state') }
            end
          end
        end
      end

      def render_health_insights
        div(class: 'space-y-4 pt-4') do
          Heading(level: 2, size: '5', class: 'font-bold') { 'Smart Insights' }
          render RubyUI::Card.new(
            class: 'bg-indigo-600 rounded-[2.5rem] p-8 text-white relative overflow-hidden border-none ' \
                   'transition-all duration-300 hover:scale-[1.02] hover:shadow-xl hover:shadow-indigo-500/20 ' \
                   'group cursor-default'
          ) do
            div(class: 'absolute -right-8 -top-8 w-32 h-32 bg-white/10 rounded-full blur-2xl ' \
                       'group-hover:bg-white/20 transition-all')
            div(class: 'relative z-10') do
              div(class: 'w-10 h-10 rounded-xl bg-white/20 flex items-center justify-center mb-6 ' \
                         'group-hover:scale-110 transition-transform') do
                render Icons::AlertCircle.new(size: 20)
              end
              Heading(level: 3, size: '4', class: 'font-bold mb-2') { 'Pattern detected' }
              Text(class: 'text-indigo-100 text-sm leading-relaxed mb-6') do
                "You've been remarkably consistent this week! Great job staying on track with your core regimen."
              end
              button(
                class: 'text-xs font-bold uppercase tracking-widest text-white border-b border-white/30 ' \
                       'pb-1 hover:border-white transition-all'
              ) do
                'View Full Report'
              end
            end
          end
        end
      end

      def render_prescriptions_section
        return if active_prescriptions.empty?

        div(class: 'space-y-6') do
          Heading(level: 2, size: '5', class: 'font-bold') { t('dashboard.medication_schedule') }
          div(class: 'space-y-4') do
            active_prescriptions.take(3).each do |prescription|
              upcoming_dose_item(prescription)
            end
          end
        end
      end

      def upcoming_dose_item(prescription)
        div(class: 'flex items-start gap-3 p-1') do
          div(class: 'w-2 h-2 rounded-full bg-primary mt-2 flex-shrink-0')
          div do
            Text(weight: 'bold', size: '3') { prescription.medicine.name }
            div(class: 'flex gap-2 items-center') do
              Text(size: '2', weight: 'muted') do
                "#{prescription.dosage.amount} #{prescription.dosage.unit} — #{prescription.dosage.frequency}"
              end
            end
          end
        end
      end

      def render_supply_levels
        div(class: 'space-y-6') do
          Heading(level: 2, size: '5', class: 'font-bold') { 'Stock Inventory' }
          render RubyUI::Card.new(
            class: 'bg-white p-8 rounded-[2.5rem] border border-slate-100 shadow-sm transition-all ' \
                   'duration-300 hover:shadow-md hover:scale-[1.01] cursor-default'
          ) do
            div(class: 'space-y-6') do
              active_prescriptions.take(3).each do |p|
                # Use current_supply as the numerator and stock as the denominator (capacity)
                # Note: We ensure stock is at least 1 to avoid division by zero
                render_supply_item(p.medicine)
              end
              render RubyUI::Link.new(
                href: medicines_path,
                variant: :ghost,
                class: 'w-full py-4 rounded-2xl bg-slate-50 text-slate-500 text-xs font-bold ' \
                       'hover:bg-slate-100 transition-all uppercase tracking-widest no-underline flex justify-center'
              ) do
                'Order Refills'
              end
            end
          end
        end
      end

      def render_supply_item(medicine)
        current = medicine.current_supply || 0
        total = [medicine.stock || 1, 1].max
        percentage = (current.to_f / total * 100).round

        div(class: 'space-y-2') do
          div(class: 'flex justify-between items-center text-xs') do
            span(class: 'font-bold') { medicine.name }
            span(class: 'text-slate-400 font-bold') { "#{current} left" }
          end
          div(class: 'h-2 w-full bg-slate-100 rounded-full overflow-hidden') do
            div(
              class: "h-full #{medicine.low_stock? ? 'bg-destructive' : 'bg-primary'} " \
                     'rounded-full transition-all duration-1000',
              style: "width: #{percentage}%"
            )
          end
        end
      end
    end
  end
end
