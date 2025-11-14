# frozen_string_literal: true

module Components
  module Admin
    module AuditLogs
      # Phlex component for rendering the audit logs index page
      # Displays a filterable table of all audit trail entries
      # @see docs/audit-trail.md
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::FormWith

        # Models that have audit trail enabled
        AUDITED_MODELS = %w[User Person CarerRelationship MedicationTake].freeze
        # Available event types for filtering
        EVENT_TYPES = %w[create update destroy].freeze

        attr_reader :versions, :filter_params

        def initialize(versions:, filter_params: {})
          @versions = versions
          @filter_params = filter_params
          @user_cache = {}
          super()
        end

        def view_template
          div(data: { testid: 'admin-audit-logs' }, class: 'space-y-8') do
            render_header
            render_filter_form
            render_versions_table
          end
        end

        private

        def render_header
          header(class: 'flex items-center justify-between') do
            div(class: 'space-y-2') do
              h1(class: 'text-3xl font-semibold text-slate-900') { 'Audit Trail' }
              p(class: 'text-slate-600') { 'Complete history of all changes made in MedTracker.' }
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
            render RubyUI::Button.new(type: :submit, variant: :primary, class: 'hidden') { 'Filter' }
            render_clear_button if filters_active?
          end
        end

        def filters_active?
          filter_params.present? && (filter_params[:item_type].present? || filter_params[:event].present?)
        end

        def render_clear_button
          a(
            href: '/admin/audit_logs',
            class: 'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
                   'px-4 py-2 h-10 text-sm border border-input bg-background hover:bg-accent ' \
                   'hover:text-accent-foreground'
          ) { 'Clear' }
        end

        def input_classes
          'w-full rounded-md border border-input bg-background px-3 py-2 text-sm ' \
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring'
        end

        def render_versions_table
          if versions.empty?
            render_empty_state
          else
            div(class: 'overflow-hidden rounded-xl border border-slate-200 bg-white shadow-sm') do
              table(class: 'min-w-full divide-y divide-slate-100') do
                render_table_header
                render_table_body
              end
            end
          end
        end

        def render_empty_state
          div(class: 'rounded-xl border border-slate-200 bg-white p-12 text-center shadow-sm') do
            p(class: 'text-slate-600 text-lg') { 'No audit logs found' }
            p(class: 'text-slate-500 text-sm mt-2') { 'Try adjusting your filters' }
          end
        end

        def render_table_header
          thead(class: 'bg-slate-50') do
            tr do
              th(scope: 'col', class: 'px-6 py-3 text-left text-sm font-semibold text-slate-600') { 'Timestamp' }
              th(scope: 'col', class: 'px-6 py-3 text-left text-sm font-semibold text-slate-600') { 'Record Type' }
              th(scope: 'col', class: 'px-6 py-3 text-left text-sm font-semibold text-slate-600') { 'Event' }
              th(scope: 'col', class: 'px-6 py-3 text-left text-sm font-semibold text-slate-600') { 'User' }
              th(scope: 'col', class: 'px-6 py-3 text-left text-sm font-semibold text-slate-600') { 'IP Address' }
              th(scope: 'col', class: 'px-6 py-3 text-right text-sm font-semibold text-slate-600') { 'Actions' }
            end
          end
        end

        def render_table_body
          tbody(class: 'divide-y divide-slate-100') do
            versions.each do |version|
              render_version_row(version)
            end
          end
        end

        def render_version_row(version)
          tr(class: 'hover:bg-slate-50', data: { version_id: version.id }) do
            td(class: 'px-6 py-4 text-sm text-slate-900') do
              version.created_at.strftime('%Y-%m-%d %H:%M:%S')
            end
            td(class: 'px-6 py-4 text-sm text-slate-600') { version.item_type.titleize }
            td(class: 'px-6 py-4 text-sm') do
              render_event_badge(version.event)
            end
            td(class: 'px-6 py-4 text-sm text-slate-600') do
              render_user_info(version.whodunnit)
            end
            td(class: 'px-6 py-4 text-sm text-slate-600 font-mono') { version.ip || 'N/A' }
            td(class: 'px-6 py-4 text-sm text-right') do
              a(
                href: "/admin/audit_logs/#{version.id}",
                class: 'text-primary hover:text-primary/80 font-medium'
              ) { 'View Details' }
            end
          end
        end

        def render_event_badge(event)
          badge_class = case event
                        when 'create'
                          'bg-green-100 text-green-800'
                        when 'update'
                          'bg-blue-100 text-blue-800'
                        when 'destroy'
                          'bg-red-100 text-red-800'
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
      end
    end
  end
end
