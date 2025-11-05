# frozen_string_literal: true

# Auditable concern for tracking changes to sensitive models
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
    # Only audit if there are changes to track
    saved_changes.any? && !saved_changes.keys.all? { |k| k.in?(%w[updated_at created_at]) }
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
