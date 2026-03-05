# frozen_string_literal: true

RSpec.configure do |config|
  config.before do
    next unless defined?(Bullet)

    Bullet.start_request
  end

  config.around(:each, :bullet) do |example|
    next example.run unless defined?(Bullet)

    Bullet.raise = true
    example.run
    Bullet.perform_out_of_channel_notifications if Bullet.notification?
  ensure
    Bullet.raise = false if defined?(Bullet)
  end

  config.after do
    next unless defined?(Bullet)

    Bullet.end_request
  end
end
