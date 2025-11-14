# frozen_string_literal: true

# Configure ActiveRecord to allow Date/Time classes in YAML
# This is needed for PaperTrail to serialize date fields
ActiveRecord.yaml_column_permitted_classes = [
  Date,
  Time,
  Symbol,
  BigDecimal,
  ActiveSupport::HashWithIndifferentAccess,
  ActiveSupport::TimeWithZone,
  ActiveSupport::TimeZone
]

# PaperTrail configuration for MedTracker audit trail
PaperTrail.config.enabled = true
PaperTrail.config.has_paper_trail_defaults = {
  on: %i[create update destroy]
}
