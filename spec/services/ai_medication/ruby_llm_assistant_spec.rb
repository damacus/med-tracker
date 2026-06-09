# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AiMedication::RubyLlmAssistant do
  subject(:assistant) { described_class.new }

  let(:medication_identity) { { name: 'Calpol Six Plus', nhs_dmd_id: 'abc123' } }

  # A minimal stand-in for RubyLLM that has the methods the assistant calls.
  def stub_ruby_llm_module(chat_double)
    ruby_llm_stub = Module.new do
      def self.chat(**_opts)
        raise 'RubyLLM stub: use allow(RubyLLM).to receive(:chat)'
      end
    end
    stub_const('RubyLLM', ruby_llm_stub)
    allow(RubyLLM).to receive(:chat).and_return(chat_double)
  end

  describe '#call' do
    context 'when RubyLLM is not defined' do
      before { hide_const('RubyLLM') }

      it 'returns a suggestion with ruby_llm_unavailable error' do
        suggestion = assistant.call(medication_identity: medication_identity)

        expect(suggestion.errors).to eq(['ruby_llm_unavailable'])
        expect(suggestion).to be_empty
      end
    end

    context 'when RubyLLM is defined but no API key is configured' do
      before do
        ruby_llm_stub = Module.new
        stub_const('RubyLLM', ruby_llm_stub)
        allow(ENV).to receive(:fetch).and_call_original
        %w[OPENAI_API_KEY ANTHROPIC_API_KEY GEMINI_API_KEY AZURE_API_KEY OPENROUTER_API_KEY].each do |key|
          allow(ENV).to receive(:fetch).with(key, nil).and_return(nil)
        end
      end

      it 'returns a suggestion with ruby_llm_unconfigured error' do
        suggestion = assistant.call(medication_identity: medication_identity)

        expect(suggestion.errors).to eq(['ruby_llm_unconfigured'])
        expect(suggestion).to be_empty
      end
    end

    context 'when RubyLLM is available and configured' do
      let(:chat_double) do
        dbl = double('RubyLLM::Chat') # rubocop:disable RSpec/VerifiedDoubles
        allow(dbl).to receive(:with_instructions).and_return(dbl)
        allow(dbl).to receive(:respond_to?).with(:with_tools).and_return(false)
        dbl
      end

      let(:response_double) do
        dbl = double('RubyLLM::Message') # rubocop:disable RSpec/VerifiedDoubles
        dbl
      end

      before do
        stub_ruby_llm_module(chat_double)
        allow(ENV).to receive(:fetch).and_call_original
        allow(ENV).to receive(:fetch).with('ANTHROPIC_API_KEY', nil).and_return('test-key')
        allow(ENV).to receive(:fetch).with(described_class::MODEL_ENV, nil).and_return(nil)
        allow(chat_double).to receive(:ask).and_return(response_double)
      end

      it 'returns a Suggestion built from valid JSON content' do
        payload = {
          'medication' => { 'description' => 'Paracetamol for children' },
          'doses' => [{ 'amount' => 5, 'unit' => 'ml' }],
          'sources' => [{ 'url' => 'https://example.com', 'title' => 'Example' }],
          'errors' => []
        }
        allow(response_double).to receive(:content).and_return(payload.to_json)

        suggestion = assistant.call(medication_identity: medication_identity)

        expect(suggestion).to be_a(AiMedication::Suggestion)
        expect(suggestion.medication).to eq('description' => 'Paracetamol for children')
        expect(suggestion.doses.first).to include('amount' => 5, 'unit' => 'ml')
      end

      it 'accepts a Hash response directly without JSON parsing' do
        payload = { 'medication' => { 'description' => 'Paracetamol' }, 'doses' => [], 'sources' => [],
                    'errors' => [] }
        allow(response_double).to receive(:content).and_return(payload)

        suggestion = assistant.call(medication_identity: medication_identity)

        expect(suggestion.medication).to eq('description' => 'Paracetamol')
      end

      it 'returns invalid_model_response error when content is unparseable JSON' do
        allow(response_double).to receive(:content).and_return('this is not json {{{')

        suggestion = assistant.call(medication_identity: medication_identity)

        expect(suggestion.errors).to eq(['invalid_model_response'])
      end

      it 'sends the medication_identity JSON in the prompt to the chat' do
        allow(response_double).to receive(:content).and_return(
          { 'medication' => {}, 'doses' => [], 'sources' => [], 'errors' => [] }.to_json
        )

        assistant.call(medication_identity: medication_identity)

        expected_prompt = { task: 'Find trusted source evidence and draft medication onboarding fields.',
                            medication_identity: medication_identity }.to_json
        expect(chat_double).to have_received(:ask).with(expected_prompt)
      end

      context 'when a specific model env var is set' do
        let(:model_chat_double) do
          dbl = double('RubyLLM::Chat') # rubocop:disable RSpec/VerifiedDoubles
          allow(dbl).to receive(:with_instructions).and_return(dbl)
          allow(dbl).to receive(:respond_to?).with(:with_tools).and_return(false)
          allow(dbl).to receive(:ask).and_return(response_double)
          dbl
        end

        before do
          allow(ENV).to receive(:fetch).with(described_class::MODEL_ENV, nil).and_return('claude-3-5-sonnet-20241022')
          allow(RubyLLM).to receive(:chat).with(model: 'claude-3-5-sonnet-20241022').and_return(model_chat_double)
          allow(response_double).to receive(:content).and_return(
            { 'medication' => {}, 'doses' => [], 'sources' => [], 'errors' => [] }.to_json
          )
        end

        it 'passes the model name to RubyLLM.chat' do
          assistant.call(medication_identity: medication_identity)

          expect(RubyLLM).to have_received(:chat).with(model: 'claude-3-5-sonnet-20241022')
        end
      end

      context 'when the chat supports with_tools' do
        let(:tools_chat_double) do
          dbl = double('RubyLLM::Chat') # rubocop:disable RSpec/VerifiedDoubles
          allow(dbl).to receive(:with_instructions).and_return(dbl)
          allow(dbl).to receive(:respond_to?).with(:with_tools).and_return(true)
          allow(dbl).to receive(:with_tools).and_return(dbl)
          allow(dbl).to receive(:ask).and_return(response_double)
          dbl
        end

        before do
          allow(RubyLLM).to receive(:chat).and_return(tools_chat_double)
          allow(response_double).to receive(:content).and_return(
            { 'medication' => {}, 'doses' => [], 'sources' => [], 'errors' => [] }.to_json
          )
        end

        it 'registers tools with the chat' do
          assistant.call(medication_identity: medication_identity)

          expect(tools_chat_double).to have_received(:with_tools).with(
            an_instance_of(AiMedication::Tools::SearchMedicationSources),
            an_instance_of(AiMedication::Tools::FetchMedicationSource),
            an_instance_of(AiMedication::Tools::ExtractMedicationGuidance)
          )
        end
      end
    end
  end
end
