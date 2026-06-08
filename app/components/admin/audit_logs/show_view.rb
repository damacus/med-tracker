# frozen_string_literal: true

module Components
  module Admin
    module AuditLogs
      # Phlex component for rendering audit log detail page
      # Shows complete information about a single audit trail entry
      # @see docs/audit-trail.md
      class ShowView < Components::Base
        # Fields that should never be displayed in audit logs for security
        SENSITIVE_FIELDS = %w[password_digest password_hash token token_digest].freeze

        attr_reader :version

        def initialize(version:)
          @version = version
          super()
        end

        def view_template
          div(
            data: { testid: 'admin-audit-log-detail' },
            class: 'container mx-auto max-w-6xl space-y-6 px-4 py-8 pb-24 md:pb-8'
          ) do
            render_header
            render_version_details
            render_changes_section
            render_new_state_section
          end
        end

        private

        def render_header
          header(class: 'flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between') do
            div(class: 'space-y-1') do
              m3_heading(level: 1) { t('admin.audit_logs.show.title') }
              m3_text(weight: 'muted') { event_heading }
            end
            Link(href: '/admin/audit_logs', variant: :outlined, class: 'shrink-0 self-start') do
              t('admin.audit_logs.show.back')
            end
          end
        end

        def event_heading
          [version.item_type.titleize, record_identifier, version.event.titleize].compact.join(' - ')
        end

        def record_identifier
          "##{version.item_id}" unless version.item_id.to_i.zero?
        end

        def render_version_details
          Card(variant: :outlined, class: 'rounded-xl border border-outline bg-surface shadow-sm') do
            CardHeader(class: 'space-y-2 p-6 pb-4') do
              m3_heading(level: 2, size: '4', class: 'font-semibold leading-none tracking-tight') do
                t('admin.audit_logs.show.event_information')
              end
              m3_text(size: '2', weight: 'muted') { t('admin.audit_logs.show.event_information_description') }
            end
            CardContent(class: 'space-y-6 p-6 pt-0') do
              render_event_summary if event_summary_items.any?
              dl(class: 'grid grid-cols-1 gap-x-8 gap-y-5 sm:grid-cols-2 lg:grid-cols-3') do
                render_detail_item(t('admin.audit_logs.show.timestamp'),
                                   version.created_at.strftime('%Y-%m-%d %H:%M:%S %Z'))
                render_detail_item(t('admin.audit_logs.show.record_type'), version.item_type.titleize)
                render_detail_item(
                  t('admin.audit_logs.show.record_id'),
                  record_identifier || t('admin.audit_logs.show.na')
                )
                render_detail_item(t('admin.audit_logs.show.event_type'), version.event.titleize)
                render_detail_item(t('admin.audit_logs.show.user'), user_name)
                render_detail_item(t('admin.audit_logs.show.ip_address'), version.ip || t('admin.audit_logs.show.na'))
              end
            end
          end
        end

        def render_event_summary
          section(class: 'rounded-lg border border-outline-variant bg-surface-container-low p-4',
                  data: { testid: event_summary_testid }) do
            h3(class: 'text-sm font-semibold text-foreground') { t('admin.audit_logs.show.event_summary') }
            dl(class: 'mt-4 grid grid-cols-1 gap-x-6 gap-y-4 sm:grid-cols-2') do
              event_summary_items.each do |label, value|
                render_detail_item(label, value)
              end
            end
          end
        end

        def event_summary_testid
          version.item_type == 'MedicationTake' ? 'audit-log-medication-take-summary' : 'audit-log-event-summary'
        end

        def render_detail_item(label, value)
          div do
            dt(class: 'text-xs font-medium uppercase text-on-surface-variant') { label }
            dd(class: 'mt-1 break-words text-sm text-foreground') { value }
          end
        end

        def user_name
          return I18n.t('admin.audit_logs.show.system') if version.whodunnit.blank?

          user = User.find_by(id: version.whodunnit)
          user ? user.name : "User ##{version.whodunnit}"
        end

        def render_changes_section
          return if version.object.blank?

          Card(variant: :outlined, class: 'rounded-xl border border-outline bg-surface shadow-sm') do
            CardHeader(class: 'space-y-2 p-6 pb-4') do
              m3_heading(level: 2, size: '4', class: 'font-semibold leading-none tracking-tight') do
                object_section_title
              end
              CardDescription { object_section_description }
            end
            CardContent(class: 'p-6 pt-0') do
              pre(class: raw_payload_classes) do
                code { format_object(version.object) }
              end
            end
          end
        end

        def render_new_state_section
          return if version.event == 'destroy'

          new_state = compute_new_state
          return if new_state.blank?

          Card(variant: :outlined, class: 'rounded-xl border border-outline bg-surface shadow-sm') do
            CardHeader(class: 'space-y-2 p-6 pb-4') do
              m3_heading(level: 2, size: '4', class: 'font-semibold leading-none tracking-tight') do
                t('admin.audit_logs.show.new_state')
              end
              CardDescription { description_for_new_state }
            end
            CardContent(class: 'p-6 pt-0') do
              pre(class: raw_payload_classes) do
                code { format_new_state(new_state) }
              end
            end
          end
        end

        def object_section_title
          external_lookup? ? t('admin.audit_logs.show.audit_payload') : t('admin.audit_logs.show.previous_state')
        end

        def raw_payload_classes
          'max-h-[28rem] overflow-auto rounded-lg bg-surface-container p-4 text-xs font-mono leading-relaxed ' \
            'text-foreground'
        end

        def object_section_description
          if external_lookup?
            t('admin.audit_logs.show.audit_payload_description')
          else
            t('admin.audit_logs.show.previous_state_description')
          end
        end

        def event_summary_items
          return medication_take_summary_items if medication_take_summary_items.any?
          return [] unless external_lookup_data

          {
            I18n.t('admin.audit_logs.show.lookup') => external_lookup_data['query'],
            I18n.t('admin.audit_logs.show.result') => external_lookup_data['result_status']&.titleize,
            I18n.t('admin.audit_logs.show.matches') => external_lookup_data['result_count']&.to_s,
            I18n.t('admin.audit_logs.show.matched_guidance') => external_lookup_data['matched_title'],
            I18n.t('admin.audit_logs.show.matched_url') => external_lookup_data['matched_url']
          }.filter_map { |label, value| [label, value] if value.present? }
        end

        def medication_take_summary_items
          return [] unless medication_take_record

          medication_take_summary_values.filter_map { |label, value| [label, value] if value.present? }
        end

        def medication_take_summary_values
          {
            I18n.t('admin.audit_logs.show.medication') => medication_take_record.medication&.display_name,
            I18n.t('admin.audit_logs.show.patient') => medication_take_record.person&.name,
            I18n.t('admin.audit_logs.show.dose') => medication_take_dose,
            I18n.t('admin.audit_logs.show.administered_at') => medication_take_administered_at,
            I18n.t('admin.audit_logs.show.logged_by') => user_name,
            I18n.t('admin.audit_logs.show.stock_source') => medication_take_record.inventory_medication&.display_name
          }
        end

        def medication_take_dose
          DoseAmount.new(medication_take_record.dose_amount, medication_take_record.dose_unit).to_s
        end

        def medication_take_administered_at
          medication_take_record.taken_at&.strftime('%Y-%m-%d %H:%M %Z')
        end

        def medication_take_record
          return @medication_take_record if defined?(@medication_take_record)

          @medication_take_record = if version.item_type == 'MedicationTake' && version.item_id.present?
                                      medication_take_relation.find_by(id: version.item_id)
                                    end
        end

        def medication_take_relation
          MedicationTake.includes(:taken_from_medication, :taken_from_location,
                                  schedule: %i[medication person],
                                  person_medication: %i[medication person])
        end

        def external_lookup?
          version.item_type == ExternalLookup::AuditLogger::ITEM_TYPE
        end

        def external_lookup_data
          return nil unless external_lookup? && version.object.present?

          data = JSON.parse(version.object)
          data if data.is_a?(Hash)
        rescue JSON::ParserError
          nil
        end

        def description_for_new_state
          case version.event
          when 'create'
            I18n.t('admin.audit_logs.show.new_state_create')
          when 'update'
            I18n.t('admin.audit_logs.show.new_state_update')
          else
            I18n.t('admin.audit_logs.show.new_state_other')
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
