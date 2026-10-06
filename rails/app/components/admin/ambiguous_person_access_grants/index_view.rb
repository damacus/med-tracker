# frozen_string_literal: true

module Components
  module Admin
    module AmbiguousPersonAccessGrants
      class IndexView < Components::Base
        attr_reader :grants, :pagy_obj

        def initialize(grants:, pagy: nil)
          @grants = grants
          @pagy_obj = pagy
          super()
        end

        def view_template
          div(data: { testid: 'admin-ambiguous-person-access-grants' },
              class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl space-y-8') do
            render_header
            render_grants_table
            render_pagination if pagy_obj && pagy_obj.pages > 1
          end
        end

        private

        def render_header
          div(class: 'mb-8') do
            m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
              t('admin.ambiguous_person_access_grants.index.title')
            end
            m3_text(weight: 'muted', class: 'mt-2 block') do
              t('admin.ambiguous_person_access_grants.index.subtitle')
            end
          end
        end

        def render_grants_table
          div(class: 'rounded-xl border border-border bg-card shadow-sm overflow-hidden') do
            render RubyUI::Table.new do
              render_table_header
              render_table_body
            end
          end
        end

        def render_table_header
          render RubyUI::TableHeader.new do
            render RubyUI::TableRow.new do
              render(RubyUI::TableHead.new { t('admin.ambiguous_person_access_grants.index.table.id') })
              render(RubyUI::TableHead.new { t('admin.ambiguous_person_access_grants.index.table.carer') })
              render(RubyUI::TableHead.new { t('admin.ambiguous_person_access_grants.index.table.patient') })
              render(RubyUI::TableHead.new { t('admin.ambiguous_person_access_grants.index.table.access_level') })
              render(RubyUI::TableHead.new { t('admin.ambiguous_person_access_grants.index.table.relationship') })
              render(RubyUI::TableHead.new { t('admin.ambiguous_person_access_grants.index.table.status') })
              render(RubyUI::TableHead.new { t('admin.ambiguous_person_access_grants.index.table.created_at') })
              render(RubyUI::TableHead.new { t('admin.ambiguous_person_access_grants.index.table.expires_at') })
              render(RubyUI::TableHead.new { t('admin.ambiguous_person_access_grants.index.table.revoked_at') })
            end
          end
        end

        def render_table_body
          render RubyUI::TableBody.new do
            if grants.empty?
              render RubyUI::TableRow.new do
                render RubyUI::TableCell.new(colspan: 9, class: 'py-8 text-center text-on-surface-variant') do
                  t('admin.ambiguous_person_access_grants.index.empty')
                end
              end
            else
              grants.each { |grant| render_grant_row(grant) }
            end
          end
        end

        def render_grant_row(grant)
          render RubyUI::TableRow.new(data: { grant_id: grant.id }) do
            render RubyUI::TableCell.new(class: 'font-medium') { grant.id.to_s }
            render(RubyUI::TableCell.new { grant.household_membership.person.name })
            render(RubyUI::TableCell.new { grant.person.name })
            render(RubyUI::TableCell.new { grant.access_level.to_s.humanize })
            render(RubyUI::TableCell.new { grant.relationship_type.to_s.humanize })
            render(RubyUI::TableCell.new { relationship_status(grant) })
            render(RubyUI::TableCell.new { format_timestamp(grant.created_at) })
            render(RubyUI::TableCell.new { format_timestamp(grant.expires_at) })
            render(RubyUI::TableCell.new { format_timestamp(grant.revoked_at) })
          end
        end

        def relationship_status(grant)
          type = grant[:compatible_relationship_type].to_s.humanize
          status = if grant[:compatible_relationship_active]
                     t('admin.ambiguous_person_access_grants.index.active')
                   else
                     t('admin.ambiguous_person_access_grants.index.inactive')
                   end
          "#{type} (#{status})"
        end

        def format_timestamp(timestamp)
          timestamp ? I18n.l(timestamp, format: :short) : t('admin.ambiguous_person_access_grants.index.not_available')
        end

        def render_pagination
          nav(
            class: 'flex items-center justify-between border-t border-border bg-card px-4 py-3 sm:px-6',
            'aria-label': t('admin.ambiguous_person_access_grants.index.pagination.label')
          ) do
            render_pagination_info
            div(class: 'flex items-center gap-2') do
              render_previous_button
              render_next_button
            end
          end
        end

        def render_pagination_info
          m3_text(size: '2', class: 'text-foreground') do
            plain "#{t('admin.ambiguous_person_access_grants.index.pagination.showing')} "
            span(class: 'font-medium') { pagy_obj.from.to_s }
            plain " #{t('admin.ambiguous_person_access_grants.index.pagination.to')} "
            span(class: 'font-medium') { pagy_obj.to.to_s }
            plain " #{t('admin.ambiguous_person_access_grants.index.pagination.of')} "
            span(class: 'font-medium') { pagy_obj.count.to_s }
            plain " #{t('admin.ambiguous_person_access_grants.index.pagination.results')}"
          end
        end

        def render_previous_button
          if pagy_obj.previous
            render RubyUI::Link.new(
              href: admin_ambiguous_person_access_grants_path(page: pagy_obj.previous),
              variant: :link,
              class: pagination_button_classes
            ) { t('admin.ambiguous_person_access_grants.index.pagination.previous') }
          else
            span(class: "#{pagination_button_classes} opacity-50 cursor-not-allowed") do
              t('admin.ambiguous_person_access_grants.index.pagination.previous')
            end
          end
        end

        def render_next_button
          if pagy_obj.next
            render RubyUI::Link.new(
              href: admin_ambiguous_person_access_grants_path(page: pagy_obj.next),
              variant: :link,
              class: pagination_button_classes
            ) { t('admin.ambiguous_person_access_grants.index.pagination.next') }
          else
            span(class: "#{pagination_button_classes} opacity-50 cursor-not-allowed") do
              t('admin.ambiguous_person_access_grants.index.pagination.next')
            end
          end
        end

        def pagination_button_classes
          'relative inline-flex items-center rounded-md bg-card px-3 py-2 text-sm font-semibold ' \
            'text-foreground ring-1 ring-inset ring-border hover:bg-tertiary-container'
        end
      end
    end
  end
end
