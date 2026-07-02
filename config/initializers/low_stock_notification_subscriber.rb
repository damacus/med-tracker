# frozen_string_literal: true

ActiveSupport::Notifications.subscribe('low_stock_threshold_reached.med_tracker') do |_name, _started, _finished, _id, payload|
  LowStockNotificationJob.perform_later(payload.fetch(:household_id), payload.fetch(:medication_id), payload.fetch(:take_id))
end
