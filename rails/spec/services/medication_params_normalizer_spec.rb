# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationParamsNormalizer do
  let(:config_keys) { %i[times frequency] }

  # The normalizer mutates `permitted` in place (the caller uses .tap).
  # We call .call for its side effects and inspect the hash directly.
  def normalize(permitted)
    described_class.call(permitted, schedule_config_keys: config_keys)
    permitted
  end

  describe 'default_schedule_config normalisation' do
    it 'leaves params untouched when the key is absent' do
      params = { name: 'x' }
      normalize(params)
      expect(params).to eq(name: 'x')
    end

    it 'coerces a blank (empty string) config to an empty hash' do
      params = { default_schedule_config: '' }
      normalize(params)
      expect(params[:default_schedule_config]).to eq({})
    end

    it 'coerces a nil config to an empty hash' do
      params = { default_schedule_config: nil }
      normalize(params)
      expect(params[:default_schedule_config]).to eq({})
    end

    it 'parses a JSON string config' do
      params = { default_schedule_config: '{"frequency":"daily"}' }
      normalize(params)
      expect(params[:default_schedule_config]).to eq('frequency' => 'daily')
    end

    it 'returns an empty hash for invalid JSON' do
      params = { default_schedule_config: 'not json' }
      normalize(params)
      expect(params[:default_schedule_config]).to eq({})
    end

    it 'accepts a plain hash directly' do
      hash = { 'frequency' => 'daily' }
      params = { default_schedule_config: hash }
      normalize(params)
      expect(params[:default_schedule_config]).to eq('frequency' => 'daily')
    end

    it 'permits only the declared schedule_config_keys from ActionController::Parameters' do
      ac_params = ActionController::Parameters.new('times' => '3', 'frequency' => 'daily', 'secret' => 'x')
      params = { default_schedule_config: ac_params }
      normalize(params)
      expect(params[:default_schedule_config]).to eq('times' => '3', 'frequency' => 'daily')
      expect(params[:default_schedule_config]).not_to have_key('secret')
    end

    it 'uses the schedule_config_keys to permit fields (different keys filter differently)' do
      ac_params = ActionController::Parameters.new('times' => '3', 'frequency' => 'daily')
      params = { default_schedule_config: ac_params }
      described_class.call(params, schedule_config_keys: %i[times])
      expect(params[:default_schedule_config]).to eq('times' => '3')
      expect(params[:default_schedule_config]).not_to have_key('frequency')
    end
  end

  describe 'dosage default de-duplication' do
    it 'keeps only the last selected adult-default record' do
      records = { '0' => { default_for_adults: '1' }, '1' => { default_for_adults: '1' } }
      normalize({ dosage_records_attributes: records })
      expect(records['0'][:default_for_adults]).to eq('0')
      expect(records['1'][:default_for_adults]).to eq('1')
    end

    it 'keeps only the last selected children-default record' do
      records = { '0' => { default_for_children: '1' }, '1' => { default_for_children: '1' } }
      normalize({ dosage_records_attributes: records })
      expect(records['0'][:default_for_children]).to eq('0')
      expect(records['1'][:default_for_children]).to eq('1')
    end

    it 'de-duplicates adults and children defaults independently' do
      records = {
        '0' => { default_for_adults: '1', default_for_children: '1' },
        '1' => { default_for_adults: '1', default_for_children: '0' }
      }
      normalize({ dosage_records_attributes: records })
      expect(records['0'][:default_for_adults]).to eq('0')
      expect(records['1'][:default_for_adults]).to eq('1')
      expect(records['0'][:default_for_children]).to eq('1')
      expect(records['1'][:default_for_children]).to eq('0')
    end

    it 'ignores records marked for destruction when choosing the survivor' do
      records = { '0' => { default_for_children: '1' }, '1' => { default_for_children: '1', _destroy: '1' } }
      normalize({ dosage_records_attributes: records })
      expect(records['0'][:default_for_children]).to eq('1')
    end

    it 'treats _destroy "0" as falsy (Boolean cast) and includes the record' do
      # '0' is a non-nil truthy Ruby value but Boolean.cast('0') is false
      records = { '0' => { default_for_adults: '1', _destroy: '0' }, '1' => { default_for_adults: '1' } }
      normalize({ dosage_records_attributes: records })
      # Both records are selected (neither destroyed), so first gets cleared
      expect(records['0'][:default_for_adults]).to eq('0')
      expect(records['1'][:default_for_adults]).to eq('1')
    end

    it 'treats default value "0" as falsy (Boolean cast) and ignores the record' do
      # '0' casts to false — this record should NOT be in selected_records
      records = { '0' => { default_for_adults: '0' }, '1' => { default_for_adults: '1' } }
      normalize({ dosage_records_attributes: records })
      expect(records['0'][:default_for_adults]).to eq('0')
      expect(records['1'][:default_for_adults]).to eq('1')
    end

    it 'leaves the sole selected record unchanged' do
      records = { '0' => { default_for_adults: '1' } }
      normalize({ dosage_records_attributes: records })
      expect(records['0'][:default_for_adults]).to eq('1')
    end

    it 'leaves non-default records untouched' do
      records = { '0' => { default_for_adults: '0' }, '1' => { default_for_adults: '0' } }
      normalize({ dosage_records_attributes: records })
      expect(records['0'][:default_for_adults]).to eq('0')
      expect(records['1'][:default_for_adults]).to eq('0')
    end

    it 'handles three records and clears all but the last' do
      records = {
        '0' => { default_for_adults: '1' },
        '1' => { default_for_adults: '1' },
        '2' => { default_for_adults: '1' }
      }
      normalize({ dosage_records_attributes: records })
      expect(records['0'][:default_for_adults]).to eq('0')
      expect(records['1'][:default_for_adults]).to eq('0')
      expect(records['2'][:default_for_adults]).to eq('1')
    end

    it 'does nothing when there are no dosage records' do
      params = { name: 'x' }
      normalize(params)
      expect(params).to eq(name: 'x')
    end
  end
end
