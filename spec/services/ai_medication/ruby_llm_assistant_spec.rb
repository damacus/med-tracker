# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AiMedication::RubyLlmAssistant do
  subject(:assistant) { described_class.new }

  let(:medication_identity) { { name: 'Calpol Six Plus', nhs_dmd_id: 'abc123' } }
  let(:empty_payload_json) { { 'medication' => {}, 'doses' => [], 'sources' => [], 'errors' => [] }.to_json }

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

  def build_chat_double(with_tools: false)
    dbl = double('RubyLLM::Chat') # rubocop:disable RSpec/VerifiedDoubles
    allow(dbl).to receive(:with_instructions).and_return(dbl)
    allow(dbl).to receive(:respond_to?).with(:with_tools).and_return(with_tools)
    allow(dbl).to receive(:with_tools).and_return(dbl) if with_tools
    dbl
  end

  def build_response_double(content)
    dbl = double('RubyLLM::Message') # rubocop:disable RSpec/VerifiedDoubles
    allow(dbl).to receive(:content).and_return(content)
    dbl
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
        stub_const('RubyLLM', Module.new)
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
      before do
        allow(ENV).to receive(:fetch).and_call_original
        allow(ENV).to receive(:fetch).with('ANTHROPIC_API_KEY', nil).and_return('test-key')
        allow(ENV).to receive(:fetch).with(described_class::MODEL_ENV, nil).and_return(nil)
      end

      it 'returns a Suggestion built from valid JSON content' do
        payload = {
          'medication' => { 'description' => 'Paracetamol for children' },
          'doses' => [{ 'amount' => 5, 'unit' => 'ml' }],
          'sources' => [{ 'url' => 'https://example.com', 'title' => 'Example' }],
          'errors' => []
        }
        chat = build_chat_double
        stub_ruby_llm_module(chat)
        allow(chat).to receive(:ask).and_return(build_response_double(payload.to_json))

        suggestion = assistant.call(medication_identity: medication_identity)

        expect(suggestion).to be_a(AiMedication::Suggestion)
        expect(suggestion.medication).to eq('description' => 'Paracetamol for children')
        expect(suggestion.doses.first).to include('amount' => 5, 'unit' => 'ml')
      end

      it 'accepts a Hash response directly without JSON parsing' do
        payload = { 'medication' => { 'description' => 'Paracetamol' }, 'doses' => [], 'sources' => [], 'errors' => [] }
        chat = build_chat_double
        stub_ruby_llm_module(chat)
        allow(chat).to receive(:ask).and_return(build_response_double(payload))

        suggestion = assistant.call(medication_identity: medication_identity)

        expect(suggestion.medication).to eq('description' => 'Paracetamol')
      end

      it 'returns invalid_model_response error when content is unparseable JSON' do
        chat = build_chat_double
        stub_ruby_llm_module(chat)
        allow(chat).to receive(:ask).and_return(build_response_double('this is not json {{{'))

        suggestion = assistant.call(medication_identity: medication_identity)

        expect(suggestion.errors).to eq(['invalid_model_response'])
      end

      it 'sends the medication_identity JSON in the prompt to the chat' do
        chat = build_chat_double
        stub_ruby_llm_module(chat)
        allow(chat).to receive(:ask).and_return(build_response_double(empty_payload_json))

        assistant.call(medication_identity: medication_identity)

        expected_prompt = { task: 'Find trusted source evidence and draft medication onboarding fields.',
                            medication_identity: medication_identity }.to_json
        expect(chat).to have_received(:ask).with(expected_prompt)
      end

      it 'passes a specific model name to RubyLLM.chat when MODEL_ENV is set' do
        allow(ENV).to receive(:fetch).with(described_class::MODEL_ENV, nil).and_return('claude-3-5-sonnet-20241022')
        chat = build_chat_double
        allow(chat).to receive(:ask).and_return(build_response_double(empty_payload_json))
        stub_ruby_llm_module(chat)
        allow(RubyLLM).to receive(:chat).with(model: 'claude-3-5-sonnet-20241022').and_return(chat)

        assistant.call(medication_identity: medication_identity)

        expect(RubyLLM).to have_received(:chat).with(model: 'claude-3-5-sonnet-20241022')
      end

      it 'registers tools with the chat when with_tools is supported' do
        chat = build_chat_double(with_tools: true)
        stub_ruby_llm_module(chat)
        allow(chat).to receive(:ask).and_return(build_response_double(empty_payload_json))

        assistant.call(medication_identity: medication_identity)

        expect(chat).to have_received(:with_tools).with(
          an_instance_of(AiMedication::Tools::SearchMedicationSources),
          an_instance_of(AiMedication::Tools::FetchMedicationSource),
          an_instance_of(AiMedication::Tools::ExtractMedicationGuidance)
        )
      end
    end
  end
end
