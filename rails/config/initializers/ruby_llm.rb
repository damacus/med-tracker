# frozen_string_literal: true

begin
  require 'ruby_llm'
rescue LoadError
  nil
end

if defined?(RubyLLM)
  RubyLLM.configure do |config|
    config.openai_api_key = ENV.fetch('OPENAI_API_KEY', nil)
    config.anthropic_api_key = ENV.fetch('ANTHROPIC_API_KEY', nil)
    config.gemini_api_key = ENV.fetch('GEMINI_API_KEY', nil)
    config.openrouter_api_key = ENV.fetch('OPENROUTER_API_KEY', nil)
  end
end
