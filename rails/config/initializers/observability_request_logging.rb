# frozen_string_literal: true

Rails.application.config.after_initialize do
  Observability::RequestCompletionSubscriber.install if Rails.env.production?
end
