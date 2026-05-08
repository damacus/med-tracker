# frozen_string_literal: true

module Components
  module Admin
    module ExternalLookupAuditEvents
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::FormWith

        SOURCES = ExternalLookupAuditEvent::SOURCES
        RESULT_STATUSES = ExternalLookupAuditEvent::RESULT_STATUSES

        attr_reader :events, :filter_params, :current_page, :total_count, :per_page

        def initialize(events:, filter_params: {}, current_page: 1, total_count: 0, per_page: 50)
          @events = events
          @filter_params = filter_params
          @current_page = current_page
          @total_count = total_count
          @per_page = per_page
          @user_cache = {}
          super()
        end

        def view_template
          div(data: { testid: 'admin-external-lookup-audit-events' },
              class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl space-y-8') do
            render_header
            render_filter_form
            render_events_table
            render_pagination if total_pages > 1
          end
        end

        private

        def render_header
          header(class: 'flex items-center justify-between') do
            div(class: 'space-y-2') do
              m3_heading(level: 1) { t('admin.external_lookup_audit_events.index.title') }
              m3_text(weight: 'muted') { t('admin.external_lookup_audit_events.index.subtitle') }
            end
            Link(href: '/admin/audit_logs', variant: :outlined) do
              t('admin.external_lookup_audit_events.index.back')
            end
          end
        end

        def render_filter_form
          Card do
            CardContent(class: 'pt-6') do
              form_with(
                url: '/admin/external_lookup_audit_events',
                method: :get,
                class: 'flex gap-4 items-end',
                data: { controller: 'filter-form' }
              ) do
                render_source_filter
                render_status_filter
                render_filter_actions
              end
            end
          end
        end

        def render_source_filter
          div(class: 'w-48') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'source') do
                t('admin.external_lookup_audit_events.index.filter.source')
              end
              select(
                name: 'source',
                id: 'source',
                class: select_classes,
                data: { action: 'change->filter-form#submit' }
              ) do
                option(value: '', selected: filter_params[:source].blank?) do
                  t('admin.external_lookup_audit_events.index.filter.all_sources')
                end
                SOURCES.each do |source|
                  option(value: source, selected: filter_params[:source] == source) { source.titleize }
                end
              end
            end
          end
        end

        def render_status_filter
          div(class: 'w-48') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'result_status') do
                t('admin.external_lookup_audit_events.index.filter.result_status')
              end
              select(
                name: 'result_status',
                id: 'result_status',
                class: select_classes,
                data: { action: 'change->filter-form#submit' }
              ) do
                option(value: '', selected: filter_params[:result_status].blank?) do
                  t('admin.external_lookup_audit_events.index.filter.all_statuses')
                end
                RESULT_STATUSES.each do |status|
                  option(value: status, selected: filter_params[:result_status] == status) { status.titleize }
                end
              end
            end
          end
        end

        def render_filter_actions
          div(class: 'flex gap-2') do
            m3_button(type: :submit, variant: :filled, class: 'hidden') do
              t('admin.external_lookup_audit_events.index.filter.filter_button')
            end
            render_clear_button if filters_active?
          end
        end

        def filters_active?
          filter_params.present? && (filter_params[:source].present? || filter_params[:result_status].present?)
        end

        def render_clear_button
          Link(
            href: '/admin/external_lookup_audit_events',
            variant: :link,
            class: 'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
                   'px-4 py-2 h-10 text-sm border border-outline bg-background hover:bg-tertiary-container ' \
                   'hover:text-on-tertiary-container'
          ) { t('admin.external_lookup_audit_events.index.filter.clear_filters') }
        end

        def render_events_table
          if events.empty?
            render_empty_state
          else
            div(class: 'rounded-[2rem] border border-border bg-card shadow-sm overflow-x-auto p-4') do
              Table(class: 'min-w-[900px]') do
                render_table_header
                render_table_body
              end
            end
          end
        end

        def render_empty_state
          div(class: 'rounded-xl border border-border bg-card p-12 text-center shadow-sm') do
            m3_text(size: '4', class: 'text-on-surface-variant') do
              t('admin.external_lookup_audit_events.index.empty.no_events')
            end
            m3_text(size: '2', weight: 'muted', class: 'mt-2') do
              t('admin.external_lookup_audit_events.index.empty.adjust_filters')
            end
          end
        end

        def render_table_header
          TableHeader(class: 'bg-secondary-container') do
            TableRow do
              TableHead { t('admin.external_lookup_audit_events.index.table.timestamp') }
              TableHead { t('admin.external_lookup_audit_events.index.table.source') }
              TableHead { t('admin.external_lookup_audit_events.index.table.event') }
              TableHead { t('admin.external_lookup_audit_events.index.table.query_hash') }
              TableHead { t('admin.external_lookup_audit_events.index.table.result_status') }
              TableHead { t('admin.external_lookup_audit_events.index.table.result_count') }
              TableHead { t('admin.external_lookup_audit_events.index.table.user') }
              TableHead { t('admin.external_lookup_audit_events.index.table.ip_address') }
            end
          end
        end

        def render_table_body
          TableBody do
            events.each do |event|
              render_event_row(event)
            end
          end
        end

        def render_event_row(event)
          TableRow(class: 'hover:bg-tertiary-container') do
            TableCell(class: 'text-foreground') do
              event.created_at.strftime('%Y-%m-%d %H:%M:%S')
            end
            TableCell(class: 'text-on-surface-variant') { event.source.titleize }
            TableCell(class: 'text-on-surface-variant') { event.event.titleize }
            TableCell(class: 'text-on-surface-variant font-mono text-xs') do
              event.query_hash ? event.query_hash.first(16) : 'N/A'
            end
            TableCell do
              render_status_badge(event.result_status)
            end
            TableCell(class: 'text-on-surface-variant') { event.result_count.to_s }
            TableCell(class: 'text-on-surface-variant') do
              render_user_info(event.whodunnit)
            end
            TableCell(class: 'text-on-surface-variant font-mono') { event.ip || 'N/A' }
          end
        end

        def render_status_badge(status)
          badge_class = case status
                        when 'success'
                          'bg-success-light text-success-text'
                        when 'not_found'
                          'bg-tertiary-container text-on-tertiary-container'
                        when 'error'
                          'bg-destructive-light text-destructive-text'
                        else
                          'bg-surface-container text-foreground'
                        end

          span(class: "inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium #{badge_class}") do
            status.titleize
          end
        end

        def render_user_info(whodunnit)
          return I18n.t('admin.external_lookup_audit_events.index.system') if whodunnit.blank?

          @user_cache[whodunnit] ||= User.find_by(id: whodunnit)
          user = @user_cache[whodunnit]
          user ? user.name : "User ##{whodunnit}"
        end

        def total_pages
          return 1 if total_count.zero?

          (total_count.to_f / per_page).ceil
        end

        def render_pagination
          nav(
            class: 'flex items-center justify-between border-t border-border ' \
                   'bg-card px-4 py-3 sm:px-6',
            'aria-label': 'Pagination'
          ) do
            render_pagination_info
            render_pagination_controls
          end
        end

        def render_pagination_info
          div(class: 'hidden sm:block') do
            m3_text(size: '2', class: 'text-foreground') do
              plain "#{t('admin.external_lookup_audit_events.index.pagination.showing')} "
              span(class: 'font-medium') { first_item_number.to_s }
              plain " #{t('admin.external_lookup_audit_events.index.pagination.to')} "
              span(class: 'font-medium') { last_item_number.to_s }
              plain " #{t('admin.external_lookup_audit_events.index.pagination.of')} "
              span(class: 'font-medium') { total_count.to_s }
              plain " #{t('admin.external_lookup_audit_events.index.pagination.results')}"
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
            Link(href: pagination_url(current_page - 1), variant: :link, class: pagination_button_classes) do
              t('admin.external_lookup_audit_events.index.pagination.previous')
            end
          else
            span(class: "#{pagination_button_classes} opacity-50 cursor-not-allowed") do
              t('admin.external_lookup_audit_events.index.pagination.previous')
            end
          end
        end

        def render_next_button
          if current_page < total_pages
            Link(href: pagination_url(current_page + 1), variant: :link, class: pagination_button_classes) do
              t('admin.external_lookup_audit_events.index.pagination.next')
            end
          else
            span(class: "#{pagination_button_classes} opacity-50 cursor-not-allowed") do
              t('admin.external_lookup_audit_events.index.pagination.next')
            end
          end
        end

        def pagination_button_classes
          'relative inline-flex items-center rounded-md bg-card px-3 py-2 text-sm font-semibold ' \
            'text-foreground ring-1 ring-inset ring-border hover:bg-tertiary-container'
        end

        def pagination_url(page)
          base_params = filter_params.to_h
          view_context.admin_external_lookup_audit_events_path(base_params.merge(page: page))
        end
      end
    end
  end
end
