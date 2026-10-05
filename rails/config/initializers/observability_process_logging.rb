# frozen_string_literal: true

Rails.application.config.after_initialize do
  if Rails.env.production?
    SolidQueue.logger = Observability::DatasetLogger.new(
      $stdout,
      dataset: 'medtracker.solid_queue',
      level: Logger::WARN
    )
    ActiveJob::Base.logger = ActiveSupport::Logger.new(IO::NULL)
    Observability::JobCompletionSubscriber.install
    Observability::JobRetrySubscriber.install
  end
end
