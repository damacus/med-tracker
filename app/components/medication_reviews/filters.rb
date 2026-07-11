# frozen_string_literal: true

module Components
  module MedicationReviews
    class Filters < Components::Base
      REVIEW_STATUSES = %w[needs_review reviewed all].freeze
      PRIORITIES = %w[all discuss_soon ask_when_convenient low_confidence].freeze

      def initialize(review_status:, priority:, review_counts:, hidden_count:, show_hidden:)
        @review_status = review_status
        @priority = priority
        @review_counts = review_counts
        @hidden_count = hidden_count
        @show_hidden = show_hidden
        super()
      end

      def view_template
        render_review_tabs
        render_priority_filters
      end

      private

      attr_reader :review_status, :priority, :review_counts, :hidden_count, :show_hidden

      def render_review_tabs
        Tabs(default: review_status, class: 'mb-6', data: { testid: 'review-status-tabs' }) do
          TabsList(class: 'flex h-auto w-full justify-start gap-1 overflow-x-auto rounded-none border-b ' \
                          'border-border bg-transparent p-0') do
            REVIEW_STATUSES.each { |status| render_review_tab(status) }
          end
        end
      end

      def render_review_tab(status)
        selected = status == review_status
        TabsTrigger(
          value: status,
          as: :a,
          href: filter_path(review_status: status),
          class: 'gap-2 rounded-none border-b-2 border-transparent px-4 py-3 font-bold shadow-none ' \
                 'data-[state=active]:border-primary data-[state=active]:bg-transparent ' \
                 'data-[state=active]:text-primary data-[state=active]:shadow-none',
          data: { state: selected ? 'active' : 'inactive' },
          aria: selected ? { current: 'page' } : {}
        ) do
          span { t("medication_reviews.filters.review_statuses.#{status}") }
          Badge(variant: selected ? :primary : :secondary, size: :sm) { review_counts.fetch(status.to_sym) }
        end
      end

      def render_priority_filters
        form(
          action: medication_review_prompts_path,
          method: :get,
          class: 'flex flex-col gap-4 border-b border-border pb-6 md:flex-row md:items-end md:justify-between',
          data: { controller: 'filter-form', testid: 'review-priority-filter' }
        ) do
          input(type: :hidden, name: 'review_status', value: review_status)
          render_priority_group
          render_hidden_switch
          noscript { m3_button(type: :submit, variant: :outlined) { t('medication_reviews.filters.apply') } }
        end
      end

      def render_priority_group
        fieldset(class: 'min-w-0') do
          legend(class: 'mb-2 text-sm font-bold text-on-surface-variant') do
            t('medication_reviews.filters.priority')
          end
          div(class: 'max-w-full overflow-x-auto pb-1') do
            ToggleGroup(
              type: :single,
              name: 'priority',
              value: priority,
              variant: :outline,
              size: :lg,
              data: { action: 'click->filter-form#submit' }
            ) do |group|
              PRIORITIES.each do |value|
                group.toggle_group_item(value: value) do
                  span(class: 'sm:hidden') { t("medication_reviews.filters.short_priorities.#{value}") }
                  span(class: 'hidden sm:inline') { t("medication_reviews.filters.priorities.#{value}") }
                end
              end
            end
          end
        end
      end

      def render_hidden_switch
        div(class: 'flex flex-col items-start gap-2 md:items-end') do
          div(class: 'flex items-center gap-3') do
            label(for: 'show_hidden', class: 'text-sm font-bold') do
              t('medication_reviews.filters.include_filtered')
            end
            Switch(
              name: 'show_hidden',
              id: 'show_hidden',
              include_hidden: false,
              checked: show_hidden,
              data: { action: 'change->filter-form#submit' },
              aria: { label: t('medication_reviews.filters.include_filtered') }
            )
          end
          render_hidden_count if hidden_count.positive?
        end
      end

      def render_hidden_count
        m3_text(variant: :body_small, class: 'text-on-surface-variant', data: { testid: 'hidden-review-count' }) do
          key = show_hidden ? 'medication_reviews.included_count' : 'medication_reviews.hidden_count'
          t(key, count: hidden_count)
        end
      end

      def filter_path(review_status: self.review_status)
        medication_review_prompts_path(
          review_status: review_status,
          priority: priority,
          show_hidden: show_hidden ? '1' : nil
        )
      end
    end
  end
end
