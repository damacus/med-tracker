class CreatePushSubscriptions < ActiveRecord::Migration[8.1]
  def change
    create_table :push_subscriptions do |t|
      t.references :account, null: false, foreign_key: { deferrable: :deferred }
      t.string :endpoint, null: false
      t.string :p256dh, null: false
      t.string :auth, null: false
      t.string :user_agent

      t.timestamps

      t.index :endpoint, unique: true
    end
  end
end
