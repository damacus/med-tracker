# frozen_string_literal: true

module SmartInsights
  Result = Data.define(:primary_insight, :insights, :learning_state?, :evidence_summary)
end
