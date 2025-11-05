# frozen_string_literal: true

module Components
  module Admin
    module AuditLogs
      # Admin audit logs view
      class IndexView < Components::Base
        attr_reader :audit_logs

        def initialize(audit_logs:)
          @audit_logs = audit_logs
          super()
        end

        def view_template
          div(data: { testid: 'admin-audit-logs' }, class: 'space-y-8 p-8') do
            render_header
            render_audit_logs
          end
        end

        private

        def render_header
          header(class: 'space-y-2') do
            h1(class: 'text-3xl font-semibold text-slate-900') { 'Audit Trail' }
            p(class: 'text-slate-600') do
              'Security audit log of all sensitive actions performed in the system'
            end
          end
        end

        def render_audit_logs
          if audit_logs.any?
            div(class: 'space-y-4') do
              audit_logs.each do |audit_log|
                render_audit_log_entry(audit_log)
              end
            end
          else
            render_empty_state
          end
        end

        def render_audit_log_entry(audit_log)
          div(class: 'rounded-lg border border-slate-200 bg-white shadow-sm') do
            # Header with action and timestamp
            div(class: 'border-b border-slate-200 bg-slate-50 px-6 py-4') do
              div(class: 'flex items-center justify-between') do
                div(class: 'space-y-1') do
                  h3(class: 'text-lg font-semibold text-slate-900') do
                    audit_log.action_description
                  end
                  p(class: 'text-sm text-slate-600') do
                    plain "by #{audit_log.actor_name}"
                    plain " • "
                    time(
                      datetime: audit_log.created_at.iso8601,
                      class: 'text-slate-600'
                    ) { audit_log.created_at.strftime('%B %d, %Y at %I:%M %p') }
                  end
                end
                span(
                  class: 'rounded-full bg-blue-100 px-3 py-1 text-sm font-medium text-blue-800'
                ) do
                  audit_log.auditable_type
                end
              end
            end

            # Code block with change data
            if audit_log.change_data.present?
              div(class: 'px-6 py-4') do
                div(class: 'rounded-md bg-slate-900 p-4') do
                  pre(class: 'overflow-x-auto') do
                    code(class: 'text-sm text-slate-100') do
                      plain format_change_data(audit_log.change_data)
                    end
                  end
                end
              end
            end

            # Footer with metadata
            if audit_log.ip_address.present? || audit_log.user_agent.present?
              div(class: 'border-t border-slate-200 bg-slate-50 px-6 py-3') do
                div(class: 'flex flex-wrap gap-4 text-sm text-slate-600') do
                  if audit_log.ip_address.present?
                    div do
                      strong { 'IP: ' }
                      plain audit_log.ip_address
                    end
                  end
                  if audit_log.user_agent.present?
                    div(class: 'truncate') do
                      strong { 'User Agent: ' }
                      plain audit_log.user_agent
                    end
                  end
                end
              end
            end
          end
        end

        def render_empty_state
          div(class: 'rounded-xl border border-slate-200 bg-white p-12 text-center shadow-sm') do
            div(class: 'space-y-4') do
              h2(class: 'text-2xl font-semibold text-slate-700') { 'No audit logs yet' }
              p(class: 'text-slate-600') do
                'Audit logs will appear here as sensitive actions are performed.'
              end
            end
          end
        end

        def format_change_data(change_data)
          return '' if change_data.nil?

          # Pretty print JSON
          JSON.pretty_generate(change_data)
        rescue StandardError
          change_data.to_s
        end
      end
    end
  end
end
