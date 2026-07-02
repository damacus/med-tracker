# frozen_string_literal: true

module Components
  module Admin
    module AuditLogs
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::FormWith

        attr_reader :versions, :filter_params, :item_types, :events, :current_page, :total_count, :per_page

        def initialize(versions:, filter_params: {}, item_types: [], events: [], **pagination)
          @versions = versions
          @filter_params = filter_params
          @item_types = item_types
          @events = events
          @current_page = pagination.fetch(:current_page, 1)
          @total_count = pagination.fetch(:total_count, 0)
          @per_page = pagination.fetch(:per_page, 50)
          @actor_resolver = AuditActorResolver.new
          super()
        end

        def view_template
          div(data: { testid: 'admin-audit-logs' },
              class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl space-y-8') do
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
              m3_heading(level: 1) { t('admin.audit_logs.index.title') }
              m3_text(weight: 'muted') { t('admin.audit_logs.index.subtitle') }
            end
          end
        end

        def render_filter_form
          Card do
            CardContent(class: 'pt-6') do
              form_with(
                url: admin_audit_logs_path,
                method: :get,
                class: 'grid gap-4 sm:grid-cols-2 md:flex md:items-end',
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
          div(class: 'min-w-0 md:w-48') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'item_type') { t('admin.audit_logs.index.filter.record_type') }
              m3_select(
                name: 'item_type',
                id: 'item_type',
                size: :sm,
                data: { action: 'change->filter-form#submit' }
              ) do
                option(value: '', selected: filter_params[:item_type].blank?) do
                  t('admin.audit_logs.index.filter.all_types')
                end
                item_types.each do |type|
                  option(value: type, selected: filter_params[:item_type] == type) { type.titleize }
                end
              end
            end
          end
        end

        def render_event_type_filter
          div(class: 'min-w-0 md:w-48') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'event') { t('admin.audit_logs.index.filter.event_type') }
              m3_select(
                name: 'event',
                id: 'event',
                size: :sm,
                data: { action: 'change->filter-form#submit' }
              ) do
                option(value: '', selected: filter_params[:event].blank?) do
                  t('admin.audit_logs.index.filter.all_events')
                end
                events.each do |event|
                  option(value: event, selected: filter_params[:event] == event) { event.titleize }
                end
              end
            end
          end
        end

        def render_filter_actions
          div(class: 'flex gap-2') do
            m3_button(type: :submit, variant: :filled, class: 'hidden') do
              t('admin.audit_logs.index.filter.filter_button')
            end
            render_clear_button if filters_active?
          end
        end

        def filters_active?
          filter_params.present? && (filter_params[:item_type].present? || filter_params[:event].present?)
        end

        def render_clear_button
          Link(
            href: admin_audit_logs_path,
            variant: :link,
            class: 'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
                   'px-4 py-2 h-10 text-sm border border-outline bg-background hover:bg-tertiary-container ' \
                   'hover:text-on-tertiary-container'
          ) { t('admin.audit_logs.index.filter.clear_filters') }
        end

        def render_versions_table
          if versions.empty?
            render_empty_state
          else
            render_mobile_versions
            div(data: { testid: 'admin-audit-logs-desktop-table' }, class: 'hidden md:block') do
              div(class: 'rounded-[2rem] border border-border bg-card shadow-sm overflow-x-auto p-4') do
                Table(class: 'min-w-[800px]') do
                  render_table_header
                  render_table_body
                end
              end
            end
          end
        end

        def render_mobile_versions
          div(class: 'space-y-4 md:hidden', data: { testid: 'admin-audit-logs-mobile-list' }) do
            versions.each do |version|
              m3_card(class: 'rounded-[2rem] border border-outline-variant/40 bg-card p-5 shadow-elevation-1',
                      data: { version_id: version.id }) do
                div(class: 'space-y-4') do
                  div(class: 'flex items-start justify-between gap-3') do
                    div(class: 'min-w-0') do
                      m3_text(size: '2', weight: 'muted',
                              class: 'uppercase tracking-widest font-bold') do
                        t('admin.audit_logs.index.table.record_type')
                      end
                      m3_text(class: 'mt-1 break-words font-bold text-foreground') { version.item_type.titleize }
                    end
                    render_event_badge(version.event)
                  end

                  dl(class: 'grid grid-cols-2 gap-3 border-t border-outline-variant/30 pt-4 text-sm') do
                    render_mobile_detail(t('admin.audit_logs.index.table.timestamp'),
                                         version.created_at.strftime('%Y-%m-%d %H:%M:%S'))
                    render_mobile_detail(t('admin.audit_logs.index.table.user'), render_user_info(version.whodunnit))
                    render_mobile_detail(t('admin.audit_logs.index.table.ip_address'), version.ip || 'N/A')
                  end

                  Link(
                    href: admin_audit_log_path(version),
                    variant: :outlined,
                    size: :sm,
                    class: 'w-full rounded-xl'
                  ) { t('admin.audit_logs.index.table.view_details') }
                end
              end
            end
          end
        end

        def render_mobile_detail(label, value)
          div(class: 'min-w-0') do
            dt(class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant') { label }
            dd(class: 'mt-1 break-words font-semibold text-foreground') { value }
          end
        end

        def render_empty_state
          div(class: 'rounded-xl border border-border bg-card p-12 text-center shadow-sm') do
            m3_text(size: '4', class: 'text-on-surface-variant') { t('admin.audit_logs.index.empty.no_logs') }
            m3_text(size: '2', weight: 'muted', class: 'mt-2') { t('admin.audit_logs.index.empty.adjust_filters') }
          end
        end

        def render_table_header
          TableHeader(class: 'bg-secondary-container') do
            TableRow do
              TableHead { t('admin.audit_logs.index.table.timestamp') }
              TableHead { t('admin.audit_logs.index.table.record_type') }
              TableHead { t('admin.audit_logs.index.table.event') }
              TableHead { t('admin.audit_logs.index.table.user') }
              TableHead { t('admin.audit_logs.index.table.ip_address') }
              TableHead(class: 'text-right') { t('admin.audit_logs.index.table.actions') }
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
          TableRow(class: 'hover:bg-tertiary-container', data: { version_id: version.id }) do
            TableCell(class: 'text-foreground') do
              version.created_at.strftime('%Y-%m-%d %H:%M:%S')
            end
            TableCell(class: 'text-on-surface-variant') { version.item_type.titleize }
            TableCell do
              render_event_badge(version.event)
            end
            TableCell(class: 'text-on-surface-variant') do
              render_user_info(version.whodunnit)
            end
            TableCell(class: 'text-on-surface-variant font-mono') { version.ip || 'N/A' }
            TableCell(class: 'text-right') do
              Link(
                href: admin_audit_log_path(version),
                variant: :link,
                class: 'text-primary hover:text-primary/80 font-medium'
              ) { t('admin.audit_logs.index.table.view_details') }
            end
          end
        end

        def render_event_badge(event)
          badge_class = case event
                        when 'create'
                          'bg-success-light text-success-text'
                        when 'update'
                          'bg-tertiary-container text-on-tertiary-container'
                        when 'destroy'
                          'bg-destructive-light text-destructive-text'
                        else
                          'bg-surface-container text-foreground'
                        end

          span(class: "inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium #{badge_class}") do
            event.titleize
          end
        end

        # Renders user information with caching to prevent N+1 queries
        # @param whodunnit [String, nil] User ID from PaperTrail
        # @return [String] User name, "System", or "User #ID"
        def render_user_info(whodunnit)
          @actor_resolver.name_for(whodunnit)
        end

        # Pagination helpers
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
              plain "#{t('admin.audit_logs.index.pagination.showing')} "
              span(class: 'font-medium') { first_item_number.to_s }
              plain " #{t('admin.audit_logs.index.pagination.to')} "
              span(class: 'font-medium') { last_item_number.to_s }
              plain " #{t('admin.audit_logs.index.pagination.of')} "
              span(class: 'font-medium') { total_count.to_s }
              plain " #{t('admin.audit_logs.index.pagination.results')}"
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
            ) { t('admin.audit_logs.index.pagination.previous') }
          else
            span(class: "#{pagination_button_classes} opacity-50 cursor-not-allowed") do
              t('admin.audit_logs.index.pagination.previous')
            end
          end
        end

        def render_next_button
          if current_page < total_pages
            Link(
              href: pagination_url(current_page + 1),
              variant: :link,
              class: pagination_button_classes
            ) { t('admin.audit_logs.index.pagination.next') }
          else
            span(class: "#{pagination_button_classes} opacity-50 cursor-not-allowed") do
              t('admin.audit_logs.index.pagination.next')
            end
          end
        end

        def pagination_button_classes
          'relative inline-flex items-center rounded-md bg-card px-3 py-2 text-sm font-semibold ' \
            'text-foreground ring-1 ring-inset ring-border hover:bg-tertiary-container'
        end

        def pagination_url(page)
          # 🛡️ Sentinel: Using .to_h securely relies on params being explicitly permitted in the controller
          # Avoid .to_unsafe_h to prevent Unvalidated Data Exposure bypasses
          base_params = filter_params.to_h
          view_context.admin_audit_logs_path(base_params.merge(page: page))
        end
      end
    end
  end
end
