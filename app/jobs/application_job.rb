# frozen_string_literal: true

class ApplicationJob < ActiveJob::Base
  attr_accessor :observability_context

  retry_on ActiveRecord::Deadlocked, wait: 5.seconds, attempts: 3
  discard_on ActiveJob::DeserializationError

  before_enqueue { |job| job.send(:prepare_observability_context) }

  around_perform do |job, block|
    previous_context = Current.observability_context
    previous_job_id = Current.job_id
    job.send(:prepare_observability_context)
    Current.observability_context = job.observability_context
    Current.job_id = job.job_id

    begin
      block.call
    rescue StandardError => e
      Otel::ExceptionRecorder.record(e, source: 'job')
      raise
    ensure
      Current.observability_context = previous_context
      Current.job_id = previous_job_id
    end
  end

  def serialize
    super.tap do |job_data|
      context = observability_context&.to_propagation
      job_data['medtracker_observability_context'] = context if context
    end
  end

  def deserialize(job_data)
    super
    payload = job_data['medtracker_observability_context']
    self.observability_context = Observability::CorrelationContext.from_propagation(payload) if payload
    self
  end

  private

  def prepare_observability_context
    return if observability_context

    parent = Current.observability_context
    self.observability_context = parent ? parent.next_attempt : Observability::CorrelationContext.start.next_attempt
  end
end
