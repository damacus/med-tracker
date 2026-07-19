# frozen_string_literal: true

module Components
  module Dashboard
    class ExperimentalBaseView < IndexView
      ACTIONABLE_STATUSES = %i[available upcoming].freeze
      TERMINAL_STATUSES = %i[taken max_reached].freeze

      private

      def dashboard_rows
        @dashboard_rows ||= (routine_tasks_by_person.values.flatten + as_needed_by_person.values.flatten)
                            .sort_by { |row| task_sort_key(row) }
      end

      def actionable_rows
        @actionable_rows ||= dashboard_rows.select { |row| ACTIONABLE_STATUSES.include?(row[:status]) }
      end

      def next_task
        @next_task ||= actionable_rows.min_by do |row|
          [due_now?(row) ? 0 : 1, row[:scheduled_at] || Time.current.end_of_day, row[:source].id]
        end
      end

      def following_tasks(limit: nil)
        rows = actionable_rows.reject { |row| row.equal?(next_task) }
        limit ? rows.first(limit) : rows
      end

      def completed_entries
        return @completed_entries if defined?(@completed_entries)

        entries = today_takes_by_person.flat_map do |person, takes|
          takes.map do |take|
            {
              kind: :completed,
              person: person,
              medication: take.medication,
              dose_amount: take.dose_amount,
              dose_unit: take.dose_unit,
              scheduled_at: take.taken_at,
              status: :taken,
              take: take
            }
          end
        end

        @completed_entries = entries.sort_by { |entry| entry[:scheduled_at] }.reverse
      end

      def timeline_entries
        @timeline_entries ||= (dashboard_rows.map { |row| row.merge(kind: :task) } + completed_entries)
                              .sort_by { |entry| entry_sort_key(entry) }
      end

      def entries_for_person(person)
        timeline_entries.select { |entry| entry[:person] == person }
      end

      def grouped_timeline_entries
        timeline_entries.group_by { |entry| period_for(entry[:scheduled_at]) }
      end

      def render_period_heading(period)
        div(class: 'flex items-center gap-2 border-b border-border/70 pb-3') do
          render period_icon(period).new(size: 18, class: 'text-on-surface-variant')
          m3_text(variant: :label_large, class: 'font-black text-on-surface-variant') do
            t("dashboard.variants.periods.#{period}")
          end
        end
      end

      def render_person_identity(person, size: :sm)
        div(class: 'flex min-w-0 items-center gap-3') do
          render Components::Shared::PersonAvatar.new(person: person, size: size)
          div(class: 'min-w-0') do
            m3_text(variant: :title_medium, class: 'truncate font-black text-foreground') { person.name }
            yield if block_given?
          end
        end
      end

      def render_task_action(row, label:, variant: :filled, size: :md, class_name: '')
        render Components::Medications::TakeAction.new(
          source: row[:source],
          context: { person: row[:person], current_user: current_user },
          amount: row[:source].dose_amount,
          button: {
            label: label,
            variant: variant,
            size: size,
            icon: Icons::HandPackage,
            testid: "take-dose-#{task_dom_id(row)}",
            class: class_name,
            form_class: nil
          }
        )
      end

      def render_medication_icon(entry, size: 22, class_name: 'text-primary')
        render Components::Shared::MedicationIcon.new(
          medication: medication_for(entry),
          size: size,
          class: class_name
        )
      end

      def render_status_badge_for(entry)
        m3_badge(
          variant: status_variant(entry),
          class: 'w-fit shrink-0 self-start px-3 py-1 text-[10px] font-black uppercase'
        ) do
          status_label_for(entry)
        end
      end

      def render_empty_dashboard
        m3_card(
          variant: :filled,
          class: 'rounded-[2rem] border-2 border-dashed border-outline-variant/60 ' \
                 'bg-surface-container-low p-12 text-center'
        ) do
          render Icons::CheckCircle.new(size: 32, class: 'mx-auto mb-4 text-success')
          m3_heading(variant: :title_large, level: 2, class: 'font-black') do
            t('dashboard.variants.empty_title')
          end
          m3_text(variant: :body_medium, class: 'mt-2 text-on-surface-variant') do
            t('dashboard.empty_state')
          end
        end
      end

      def render_stock_summary(compact: false, stacked: false)
        medication = stock_medication
        return unless medication

        m3_card(
          variant: :outlined,
          class: stock_card_classes(compact)
        ) do
          div(class: stock_content_classes(stacked)) do
            div(class: 'flex min-w-0 items-center gap-4') do
              span(class: stock_icon_classes(medication)) do
                render Icons::AlertCircle.new(size: 22)
              end
              div(class: 'min-w-0') do
                m3_heading(variant: :title_medium, level: 3, class: 'truncate font-black') do
                  t('dashboard.variants.stock_to_review')
                end
                m3_text(variant: :body_medium, class: 'text-on-surface-variant') do
                  t(
                    'dashboard.variants.stock_summary',
                    medication: medication.display_name,
                    count: stock_count(medication)
                  )
                end
              end
            end
            m3_link(
              href: medications_path,
              variant: :outlined,
              size: :md,
              class: stock_link_classes(stacked)
            ) do
              t('dashboard.variants.review_stock')
            end
          end
        end
      end

      def medication_for(entry)
        entry[:medication] || entry[:source].medication
      end

      def dose_label_for(entry)
        if entry[:kind] == :completed
          DoseAmount.new(entry[:dose_amount], entry[:dose_unit]).to_s
        else
          source = entry[:source]
          source.dose_display.presence || DoseAmount.new(source.dose_amount, source.dose_unit).to_s
        end
      end

      def time_label_for(entry)
        return t('dashboard.routine.anytime') if entry[:scheduled_at].blank?

        entry[:scheduled_at].strftime('%H:%M')
      end

      def status_label_for(entry)
        return t('dashboard.statuses.taken') if entry[:kind] == :completed
        return t('dashboard.stats.now') if due_now?(entry)

        t("dashboard.statuses.#{entry[:status]}")
      end

      def status_variant(entry)
        return :tonal if entry[:kind] == :completed
        return :filled if ACTIONABLE_STATUSES.include?(entry[:status])
        return :destructive if entry[:status] == :out_of_stock

        :outlined
      end

      def take_label_for(row)
        return t('person_medications.card.take') if current_user.nil? || current_user.person == row[:person]

        t('person_medications.card.give')
      end

      def due_now?(row)
        return true if row[:status] == :available
        return false unless row[:status] == :upcoming

        row[:scheduled_at].blank? || row[:scheduled_at] <= Time.current
      end

      def period_for(time)
        return :anytime if time.blank?
        return :morning if time.hour < 12
        return :afternoon if time.hour < 18

        :evening
      end

      def period_icon(period)
        return Icons::Moon if period == :evening
        return Icons::Clock if period == :anytime

        Icons::Sun
      end

      def grouping_path(grouping)
        options = { dashboard_grouping: grouping }
        options[:dashboard_person_id] = presenter.selected_person_id if presenter.selected_person_id.present?
        dashboard_path(**options)
      end

      def task_sort_key(row)
        [row[:scheduled_at] || Time.current.end_of_day, row[:source].id]
      end

      def entry_sort_key(entry)
        [entry[:scheduled_at] || Time.current.end_of_day, entry[:kind] == :completed ? 0 : 1]
      end

      def task_dom_id(row)
        "#{row[:source].class.name.underscore}_#{row[:source].id}"
      end

      def stock_medication
        @stock_medication ||= stock_medications.find(&:low_stock?) || stock_medications.first
      end

      def stock_medications
        active_schedules.map(&:medication).uniq(&:id)
      end

      def stock_count(medication)
        ::Medications::SupplyStatusPresenter.new(medication: medication).inventory_units_label
      end

      def stock_icon_classes(medication)
        color = if medication.low_stock?
                  'bg-warning-container text-on-warning-container'
                else
                  'bg-primary-container text-on-primary-container'
                end
        "inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-shape-full #{color}"
      end

      def stock_card_classes(compact)
        return 'rounded-[1.5rem] border-border bg-surface-container-low p-5' if compact

        'rounded-[2rem] border-border bg-surface-container-low p-6'
      end

      def stock_content_classes(stacked)
        return 'flex flex-col gap-4' if stacked

        'flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between'
      end

      def stock_link_classes(stacked)
        return 'w-full shrink-0 justify-center font-black' if stacked

        'shrink-0 font-black'
      end
    end
  end
end
