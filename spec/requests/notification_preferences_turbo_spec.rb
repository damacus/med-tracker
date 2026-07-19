# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Notification preferences turbo streams' do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }

  before { sign_in(user) }

  describe 'PATCH /notification_preference' do
    it 'returns turbo_stream and replaces the notifications card and flash' do
      patch notification_preference_path,
            params: {
              notification_preference: {
                enabled: '0',
                morning_time: '07:30',
                afternoon_time: '13:30',
                evening_time: '18:30',
                night_time: '22:30'
              }
            },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="notifications-card"')
      expect(response.body).to include('target="flash"')
      expect(user.person.notification_preference.reload).not_to be_enabled
    end

    it 'redirects to the profile after an HTML update' do
      patch notification_preference_path,
            params: {
              notification_preference: {
                enabled: '1',
                dose_due_enabled: '1',
                missed_dose_enabled: '0',
                low_stock_enabled: '1',
                private_text_enabled: '0'
              }
            }

      expect(response).to redirect_to(profile_path)
      expect(flash[:notice]).to eq(I18n.t('notification_preferences.updated'))
      expect(user.person.notification_preference.reload).to be_enabled
    end

    it 'updates optional missed-dose notifications for managed adults' do
      household = user.person.household
      membership = household.household_memberships.active.find_by!(account: user.person.account)
      selected_grant = membership.person_access_grants.active.manage.find_by!(person: people(:jane))
      unselected_grant = membership.person_access_grants.active.manage.find_by!(person: people(:bob))
      selected_grant.update!(missed_dose_notifications_enabled: false)
      unselected_grant.update!(missed_dose_notifications_enabled: true)

      patch notification_preference_path,
            params: {
              notification_preference: {
                enabled: '1',
                missed_dose_enabled: '1',
                managed_person_ids: ['', selected_grant.person_id.to_s]
              }
            }

      expect(response).to redirect_to(profile_path)
      expect(selected_grant.reload).to be_missed_dose_notifications_enabled
      expect(unselected_grant.reload).not_to be_missed_dose_notifications_enabled
    end

    it 'returns turbo stream validation feedback when the preference cannot be saved' do
      account = user.person.account
      preference = user.person.notification_preference || user.person.create_notification_preference!
      allow(Account).to receive(:find_by).and_call_original
      allow(Account).to receive(:find_by).with(id: account.id).and_return(account)
      allow(user.person).to receive(:notification_preference).and_return(preference)
      allow(preference).to receive(:update).and_return(false)

      patch notification_preference_path,
            params: { notification_preference: { enabled: '1' } },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="notifications-card"')
      expect(response.body).to include(I18n.t('notification_preferences.update_failed'))
    end
  end
end
