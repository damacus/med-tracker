# frozen_string_literal: true

module Components
  module Admin
    module AuditLogs
      # Phlex component for rendering audit log detail page
      # Shows complete information about a single audit trail entry
      # @see docs/audit-trail.md
      class ShowView < Components::Base
        # Fields that should never be displayed in audit logs for security
        SENSITIVE_FIELDS = %w[password_digest password_hash].freeze

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
            render_new_state_section
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

        def render_new_state_section
          # For create events, show the created record
          # For update events, show the current state or use object_changes
          # For destroy events, there is no new state
          return if version.event == 'destroy'

          new_state = compute_new_state
          return if new_state.blank?

          Card do
            CardHeader do
              CardTitle { 'New State' }
              CardDescription { description_for_new_state }
            end
            CardContent do
              pre(class: 'bg-slate-50 p-4 rounded-lg overflow-x-auto text-xs font-mono') do
                code { format_new_state(new_state) }
              end
            end
          end
        end

        def description_for_new_state
          case version.event
          when 'create'
            'The state of the record when it was created'
          when 'update'
            'The state of the record after this change'
          else
            'The current state of the record'
          end
        end

        def compute_new_state
          # Try to get the next version's object (which represents state after this change)
          next_version = PaperTrail::Version
                         .where(item_type: version.item_type, item_id: version.item_id)
                         .where('id > ?', version.id)
                         .order(:id)
                         .first

          if next_version&.object.present?
            # The next version's object is the state after this version's change
            next_version.object
          else
            # No next version - try to load the current record
            current_record
          end
        end

        def current_record
          model_class = version.item_type.safe_constantize
          return nil unless model_class

          record = model_class.find_by(id: version.item_id)
          return nil unless record

          # Convert to hash, excluding sensitive fields
          record.attributes.except(*SENSITIVE_FIELDS)
        rescue StandardError
          nil
        end

        def format_new_state(state)
          case state
          when String
            # YAML from PaperTrail
            format_object(state)
          when Hash
            # Hash from current record
            JSON.pretty_generate(state)
          else
            state.to_s
          end
        end

        # Formats YAML object data as pretty-printed JSON
        # Filters out sensitive fields like password_digest
        # Falls back to raw YAML if parsing fails
        # @param object_yaml [String] YAML-serialized object data
        # @return [String] Formatted JSON or original YAML
        def format_object(object_yaml)
          parsed = YAML.safe_load(object_yaml, permitted_classes: ActiveRecord.yaml_column_permitted_classes)
          filtered = filter_sensitive_fields(parsed)
          JSON.pretty_generate(filtered)
        rescue StandardError => e
          # If YAML parsing fails, return original string
          # This can happen with corrupted data or schema changes
          Rails.logger.warn("Failed to parse audit log object: #{e.message}")
          object_yaml
        end

        # Removes sensitive fields from audit data
        # @param data [Hash] The parsed audit data
        # @return [Hash] Data with sensitive fields removed
        def filter_sensitive_fields(data)
          return data unless data.is_a?(Hash)

          data.except(*SENSITIVE_FIELDS)
        end
      end
    end
  end
end
