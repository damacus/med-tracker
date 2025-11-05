# frozen_string_literal: true

# AuditLog captures all sensitive actions in the application for security and compliance
class AuditLog < ApplicationRecord
  belongs_to :user, optional: true

  validates :action, presence: true
  validates :auditable_type, presence: true
  validates :auditable_id, presence: true

  # Serialize change_data as JSON
  serialize :change_data, coder: JSON

  # Scopes for filtering
  scope :recent, -> { order(created_at: :desc) }
  scope :by_action, ->(action) { where(action: action) }
  scope :by_user, ->(user) { where(user: user) }
  scope :by_auditable, ->(type, id) { where(auditable_type: type, auditable_id: id) }

  # Human-readable action descriptions
  def action_description
    case action
    when 'create'
      "Created #{auditable_type}"
    when 'update'
      "Updated #{auditable_type}"
    when 'destroy'
      "Deleted #{auditable_type}"
    when 'take_medicine'
      'Took medication'
    else
      action.humanize
    end
  end

  # Return the user who performed the action or 'System' if no user
  def actor_name
    user&.name || 'System'
  end
end
