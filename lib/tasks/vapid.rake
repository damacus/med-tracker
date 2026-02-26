# frozen_string_literal: true

namespace :vapid do
  desc 'Generate VAPID key pair for web push notifications'
  task generate: :environment do
    require 'web_push'
    pair = WebPush.generate_key_pair
    puts 'Add to Rails credentials (rails credentials:edit):'
    puts ''
    puts 'vapid:'
    puts "  public_key: #{pair[:public_key]}"
    puts "  private_key: #{pair[:private_key]}"
  end
end
