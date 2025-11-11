# frozen_string_literal: true

# Auditable concern for tracking changes to sensitive models.
#
# Usage:
#   - Automatically creates audit logs for create, update, and destroy actions on the including model.
#   - Requires `Current.user` and `Current.request` to be set for user, IP address, and user agent tracking.
#   - Relies on the presence of an `AuditLog` model with appropriate fields.
#   - Logs the action, changed data, user, IP address, and user agent.
#   - Only audits changes beyond timestamp updates.
#
# Ensure that the including model and application context provide these dependencies.
module Auditable
  extend ActiveSupport::Concern

  included do
    after_create :audit_create
    after_update :audit_update
    after_destroy :audit_destroy
  end

  private

  def audit_create
    create_audit_log('create', auditable_changes)
  end

  def audit_update
    return unless audit_relevant_changes?

    create_audit_log('update', auditable_changes)
  end

  def audit_destroy
    create_audit_log('destroy', auditable_changes)
  end

  def audit_relevant_changes?
    # Only audit if there are changes to track beyond just timestamps
    has_non_timestamp_changes?
  end

  def has_non_timestamp_changes?
    saved_changes.keys.any? { |k| !k.in?(%w[updated_at created_at]) }
  end

  def auditable_changes
    if destroyed?
      attributes
    elsif persisted?
      saved_changes
    else
      changes
    end
  end

  def create_audit_log(action, changes_data)
    AuditLog.create(
      user: current_user,
      action: action,
      auditable_type: self.class.name,
      auditable_id: id,
      change_data: changes_data,
      ip_address: current_ip_address,
      user_agent: current_user_agent
    )
  rescue StandardError => e
    Rails.logger.error "Failed to create audit log: #{e.message}"
  end

  def current_user
    Current.user
  end

  def current_ip_address
    Current.request&.remote_ip
  end

  def current_user_agent
    Current.request&.user_agent
  end
end
