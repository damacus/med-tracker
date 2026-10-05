class AddApnsEnvironmentToNativeDeviceTokens < ActiveRecord::Migration[8.1]
  def change
    add_column :native_device_tokens, :apns_environment, :string
  end
end
