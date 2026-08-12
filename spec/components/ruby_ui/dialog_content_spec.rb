# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::DialogContent, type: :component do
  it 'uses token-driven floating surfaces instead of hard-coded white' do
    rendered = render_inline(described_class.new(size: :md) { 'Dialog body' })
    html = rendered.to_html

    expect(html).to include('bg-popover')
    expect(html).to include('bg-foreground/10')
    expect(html).to include('backdrop-blur-[1.5px]')
    expect(html).not_to include('bg-white')
    expect(html).not_to include('bg-background/80')
  end

  it 'names the close button and hides redundant close content' do
    rendered = render_inline(described_class.new(size: :md) { 'Dialog body' })
    close_button = rendered.at_css(%(button[aria-label="#{I18n.t('ruby_ui.common.close')}"]))

    expect(close_button).to be_present
    expect(close_button.at_css('svg[aria-hidden="true"]')).to be_present
    expect(close_button.css('.sr-only')).to be_empty
  end

  it 'renders a named modal dialog surface' do
    body = [
      RubyUI::DialogTitle.new { 'Record dose' }.render_in(view_context),
      RubyUI::DialogDescription.new { 'Confirm the dose details.' }.render_in(view_context)
    ]
    rendered = render_inline(described_class.new(size: :md) { view_context.safe_join(body) })
    dialog = rendered.at_css('dialog[role="dialog"]')

    expect(dialog).to be_present
    expect([dialog['aria-modal'], dialog['aria-labelledby'], dialog['aria-describedby']]).to eq(
      ['true', nil, nil]
    )
    labelled_nodes = dialog.css('[data-ruby-ui-dialog-title], [data-ruby-ui-dialog-description]')
    expect(labelled_nodes.map(&:text)).to eq(
      ['Record dose', 'Confirm the dose details.']
    )
  end

  it 'does not reference a title or description that was not rendered' do
    rendered = render_inline(described_class.new(size: :md) { 'Dialog body' })
    dialog = rendered.at_css('[role="dialog"]')

    expect(dialog['aria-labelledby']).to be_nil
    expect(dialog['aria-describedby']).to be_nil
  end

  it 'uses native cancellation rather than a template portal' do
    rendered = render_inline(described_class.new(size: :md) { 'Dialog body' })

    expect(rendered.at_css('template')).to be_nil
    expect(rendered.at_css('dialog')['data-action']).to include(
      'cancel->ruby-ui--dialog#dismiss',
      'click->ruby-ui--dialog#backdropClick',
      'keydown->ruby-ui--dialog#trapFocus'
    )
  end

  it 'does not render duplicate title and description ids across dialogs' do
    contents = 2.times.map do
      title = RubyUI::DialogTitle.new { 'Record dose' }.render_in(view_context)
      description = RubyUI::DialogDescription.new { 'Confirm the dose details.' }.render_in(view_context)

      described_class.new(size: :md) { view_context.safe_join([title, description]) }.render_in(view_context)
    end
    ids = Nokogiri::HTML.fragment(contents.join).css('[id]').pluck('id')

    expect(ids).to eq(ids.uniq)
  end

  describe RubyUI::DialogTrigger do
    it 'marks dialog triggers as nested overlay triggers' do
      rendered = render_inline(described_class.new { 'Open dialog' })

      expect(rendered.at_css('[data-ruby-ui-overlay-trigger]')).to be_present
    end
  end

  describe RubyUI::AlertDialogTrigger do
    it 'marks alert dialog triggers as nested overlay triggers' do
      rendered = render_inline(described_class.new { 'Open alert dialog' })

      expect(rendered.at_css('[data-ruby-ui-overlay-trigger]')).to be_present
    end
  end
end
