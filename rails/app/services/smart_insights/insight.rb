# frozen_string_literal: true

module SmartInsights
  Insight = Data.define(:key, :family, :severity, :title, :summary, :detail, :metric_label, :metric_value, :cta_path)
end
