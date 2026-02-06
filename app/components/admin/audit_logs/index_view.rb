# frozen_string_literal: true

module Components
  module Admin
    module AuditLogs
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::FormWith

        # Models that have audit trail enabled
        AUDITED_MODELS = %w[User Person CarerRelationship MedicationTake].freeze
        # Available event types for filtering
        EVENT_TYPES = %w[create update destroy].freeze

        attr_reader :versions, :filter_params, :current_page, :total_count, :per_page

        def initialize(versions:, filter_params: {}, current_page: 1, total_count: 0, per_page: 50)
          @versions = versions
          @filter_params = filter_params
          @current_page = current_page
          @total_count = total_count
          @per_page = per_page
          @user_cache = {}
          super()
        end

        def view_template
          div(data: { testid: 'admin-audit-logs' }, class: 'space-y-8') do
            render_header
            render_filter_form
            render_versions_table
            render_pagination if total_pages > 1
          end
        end

        private

        def render_header
          header(class: 'flex items-center justify-between') do
            div(class: 'space-y-2') do
              Heading(level: 1) { 'Audit Trail' }
              Text(weight: 'muted') { 'Complete history of all changes made in MedTracker.' }
            end
          end
        end

        def render_filter_form
          Card do
            CardContent(class: 'pt-6') do
              form_with(
                url: '/admin/audit_logs',
                method: :get,
                class: 'flex gap-4 items-end',
                data: { controller: 'filter-form' }
              ) do
                render_item_type_filter
                render_event_type_filter
                render_filter_actions
              end
            end
          end
        end

        def render_item_type_filter
          div(class: 'w-48') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'item_type') { 'Record Type' }
              select(
                name: 'item_type',
                id: 'item_type',
                class: input_classes,
                data: { action: 'change->filter-form#submit' }
              ) do
                option(value: '', selected: filter_params[:item_type].blank?) { 'All Types' }
                AUDITED_MODELS.each do |type|
                  option(value: type, selected: filter_params[:item_type] == type) { type.titleize }
                end
              end
            end
          end
        end

        def render_event_type_filter
          div(class: 'w-48') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'event') { 'Event Type' }
              select(
                name: 'event',
                id: 'event',
                class: input_classes,
                data: { action: 'change->filter-form#submit' }
              ) do
                option(value: '', selected: filter_params[:event].blank?) { 'All Events' }
                EVENT_TYPES.each do |event|
                  option(value: event, selected: filter_params[:event] == event) { event.titleize }
                end
              end
            end
          end
        end

        def render_filter_actions
          div(class: 'flex gap-2') do
            Button(type: :submit, variant: :primary, class: 'hidden') { 'Filter' }
            render_clear_button if filters_active?
          end
        end

        def filters_active?
          filter_params.present? && (filter_params[:item_type].present? || filter_params[:event].present?)
        end

        def render_clear_button
          Link(
            href: '/admin/audit_logs',
            variant: :link,
            class: 'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
                   'px-4 py-2 h-10 text-sm border border-input bg-background hover:bg-accent ' \
                   'hover:text-accent-foreground'
          ) { 'Clear filters' }
        end

        def input_classes
          'w-full rounded-md border border-input bg-background px-3 py-2 text-sm ' \
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring'
        end

        def render_versions_table
          if versions.empty?
            render_empty_state
          else
            div(class: 'rounded-xl border border-border bg-card shadow-sm') do
              Table do
                render_table_header
                render_table_body
              end
            end
          end
        end

        def render_empty_state
          div(class: 'rounded-xl border border-slate-200 bg-white p-12 text-center shadow-sm') do
            Text(size: '4', class: 'text-slate-600') { 'No audit logs found' }
            Text(size: '2', weight: 'muted', class: 'mt-2') { 'Try adjusting your filters' }
          end
        end

        def render_table_header
          TableHeader(class: 'bg-slate-50') do
            TableRow do
              TableHead { 'Timestamp' }
              TableHead { 'Record Type' }
              TableHead { 'Event' }
              TableHead { 'User' }
              TableHead { 'IP Address' }
              TableHead(class: 'text-right') { 'Actions' }
            end
          end
        end

        def render_table_body
          TableBody do
            versions.each do |version|
              render_version_row(version)
            end
          end
        end

        def render_version_row(version)
          TableRow(class: 'hover:bg-slate-50', data: { version_id: version.id }) do
            TableCell(class: 'text-slate-900') do
              version.created_at.strftime('%Y-%m-%d %H:%M:%S')
            end
            TableCell(class: 'text-slate-600') { version.item_type.titleize }
            TableCell do
              render_event_badge(version.event)
            end
            TableCell(class: 'text-slate-600') do
              render_user_info(version.whodunnit)
            end
            TableCell(class: 'text-slate-600 font-mono') { version.ip || 'N/A' }
            TableCell(class: 'text-right') do
              Link(
                href: "/admin/audit_logs/#{version.id}",
                variant: :link,
                class: 'text-primary hover:text-primary/80 font-medium'
              ) { 'View Details' }
            end
          end
        end

        def render_event_badge(event)
          badge_class = case event
                        when 'create'
                          'bg-success-light text-success-text'
                        when 'update'
                          'bg-accent text-accent-foreground'
                        when 'destroy'
                          'bg-destructive-light text-destructive-text'
                        else
                          'bg-slate-100 text-slate-800'
                        end

          span(class: "inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium #{badge_class}") do
            event.titleize
          end
        end

        # Renders user information with caching to prevent N+1 queries
        # @param whodunnit [String, nil] User ID from PaperTrail
        # @return [String] User name, "System", or "User #ID"
        def render_user_info(whodunnit)
          return 'System' if whodunnit.blank?

          # Cache user lookups to prevent N+1 queries when rendering multiple rows
          @user_cache[whodunnit] ||= User.find_by(id: whodunnit)
          user = @user_cache[whodunnit]

          user ? user.name : "User ##{whodunnit}"
        end

        # Pagination helpers
        def total_pages
          return 1 if total_count.zero?

          (total_count.to_f / per_page).ceil
        end

        def render_pagination
          nav(class: 'flex items-center justify-between border-t border-slate-200 bg-white px-4 py-3 sm:px-6',
              'aria-label': 'Pagination') do
            render_pagination_info
            render_pagination_controls
          end
        end

        def render_pagination_info
          div(class: 'hidden sm:block') do
            Text(size: '2', class: 'text-slate-700') do
              plain 'Showing '
              span(class: 'font-medium') { first_item_number.to_s }
              plain ' to '
              span(class: 'font-medium') { last_item_number.to_s }
              plain ' of '
              span(class: 'font-medium') { total_count.to_s }
              plain ' results'
            end
          end
        end

        def first_item_number
          ((current_page - 1) * per_page) + 1
        end

        def last_item_number
          [current_page * per_page, total_count].min
        end

        def render_pagination_controls
          div(class: 'flex flex-1 justify-between sm:justify-end gap-2') do
            render_previous_button
            render_next_button
          end
        end

        def render_previous_button
          if current_page > 1
            Link(
              href: pagination_url(current_page - 1),
              variant: :link,
              class: pagination_button_classes
            ) { 'Previous' }
          else
            span(class: "#{pagination_button_classes} opacity-50 cursor-not-allowed") { 'Previous' }
          end
        end

        def render_next_button
          if current_page < total_pages
            Link(
              href: pagination_url(current_page + 1),
              variant: :link,
              class: pagination_button_classes
            ) { 'Next' }
          else
            span(class: "#{pagination_button_classes} opacity-50 cursor-not-allowed") { 'Next' }
          end
        end

        def pagination_button_classes
          'relative inline-flex items-center rounded-md bg-white px-3 py-2 text-sm font-semibold ' \
            'text-slate-900 ring-1 ring-inset ring-slate-300 hover:bg-slate-50'
        end

        def pagination_url(page)
          base_params = filter_params.respond_to?(:to_unsafe_h) ? filter_params.to_unsafe_h : filter_params.to_h
          Rails.application.routes.url_helpers.admin_audit_logs_path(base_params.merge(page: page))
        end
      end
    end
  end
end
