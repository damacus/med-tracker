# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AccountWebauthnKey do
  describe 'callbacks' do
    describe '#set_default_nickname' do
      let(:account) { create(:account) }

      context 'when nickname is not provided' do
        it 'sets the default nickname to Passkey 1 for the first key' do
          key = create(:account_webauthn_key, account: account, nickname: nil)
          expect(key.nickname).to eq('Passkey 1')
        end

        it 'sets the default nickname to Passkey 2 for the second key' do
          create(:account_webauthn_key, account: account)
          key2 = create(:account_webauthn_key, account: account, nickname: nil)
          expect(key2.nickname).to eq('Passkey 2')
        end
      end

      context 'when nickname is provided' do
        it 'does not overwrite the provided nickname' do
          key = create(:account_webauthn_key, account: account, nickname: 'My Custom Key')
          expect(key.nickname).to eq('My Custom Key')
        end
      end
    end
  end
end
