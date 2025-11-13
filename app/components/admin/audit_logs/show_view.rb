# frozen_string_literal: true

module Components
  module Admin
    module AuditLogs
      # Phlex component for rendering audit log detail page
      # Shows complete information about a single audit trail entry
      # @see docs/audit-trail.md
      class ShowView < Components::Base
        attr_reader :version

        def initialize(version:)
          @version = version
          super()
        end

        def view_template
          div(data: { testid: 'admin-audit-log-detail' }, class: 'space-y-8') do
            render_header
            render_version_details
            render_changes_section
          end
        end

        private

        def render_header
          header(class: 'flex items-center justify-between') do
            div(class: 'space-y-2') do
              h1(class: 'text-3xl font-semibold text-slate-900') { 'Audit Log Details' }
              p(class: 'text-slate-600') do
                "#{version.item_type} ##{version.item_id} - #{version.event.titleize}"
              end
            end
            a(
              href: '/admin/audit_logs',
              class: 'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
                     'px-4 py-2 h-10 text-sm border border-input bg-background hover:bg-accent ' \
                     'hover:text-accent-foreground'
            ) { '← Back to Audit Logs' }
          end
        end

        def render_version_details
          Card do
            CardHeader do
              CardTitle { 'Event Information' }
            end
            CardContent do
              dl(class: 'grid grid-cols-1 gap-4 sm:grid-cols-2') do
                render_detail_item('Timestamp', version.created_at.strftime('%Y-%m-%d %H:%M:%S %Z'))
                render_detail_item('Record Type', version.item_type.titleize)
                render_detail_item('Record ID', version.item_id.to_s)
                render_detail_item('Event Type', version.event.titleize)
                render_detail_item('User', user_name)
                render_detail_item('IP Address', version.ip || 'N/A')
              end
            end
          end
        end

        def render_detail_item(label, value)
          div do
            dt(class: 'text-sm font-medium text-slate-500') { label }
            dd(class: 'mt-1 text-sm text-slate-900') { value }
          end
        end

        def user_name
          return 'System' if version.whodunnit.blank?

          user = User.find_by(id: version.whodunnit)
          user ? user.name : "User ##{version.whodunnit}"
        end

        def render_changes_section
          return if version.object.blank?

          Card do
            CardHeader do
              CardTitle { 'Previous State' }
              CardDescription { 'The state of the record before this change' }
            end
            CardContent do
              pre(class: 'bg-slate-50 p-4 rounded-lg overflow-x-auto text-xs font-mono') do
                code { format_object(version.object) }
              end
            end
          end
        end

        # Formats YAML object data as pretty-printed JSON
        # Falls back to raw YAML if parsing fails
        # @param object_yaml [String] YAML-serialized object data
        # @return [String] Formatted JSON or original YAML
        def format_object(object_yaml)
          parsed = YAML.safe_load(object_yaml, permitted_classes: ActiveRecord.yaml_column_permitted_classes)
          JSON.pretty_generate(parsed)
        rescue StandardError => e
          # If YAML parsing fails, return original string
          # This can happen with corrupted data or schema changes
          Rails.logger.warn("Failed to parse audit log object: #{e.message}")
          object_yaml
        end
      end
    end
  end
end
