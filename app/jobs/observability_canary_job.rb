# frozen_string_literal: true

class ObservabilityCanaryJob < ApplicationJob
  queue_as :default

  def perform
    Observability::DeployedCanary.emit(kind: :job)
  end
end
