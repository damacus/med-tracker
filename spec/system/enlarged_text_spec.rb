# frozen_string_literal: true

require 'rails_helper'

module EnlargedTextGeometry
end

RSpec.describe 'Enlarged text layout', :js do
  fixtures :all

  before do
    sign_in(users(:admin))
    page.current_window.resize_to(320, 844)
    visit profile_path
  end

  it 'keeps shell and notice content inside their bounds at ordinary and enlarged text sizes',
     :aggregate_failures do
    [16, 32].each do |font_size|
      page.execute_script("document.documentElement.style.fontSize = '#{font_size}px'")
      geometry = page.evaluate_script(shell_geometry_script)

      expect(geometry.fetch('menu')).to include('withinViewport' => true, 'textContained' => true),
                                        "font=#{font_size} geometry=#{geometry.inspect}"
      expect(geometry.fetch('search')).to include('withinViewport' => true, 'textContained' => true),
                                          "font=#{font_size} geometry=#{geometry.inspect}"
      brand = geometry.fetch('brand')
      expect(brand).to include('withinViewport' => true, 'text' => 'MedTracker'),
                       "font=#{font_size} geometry=#{geometry.inspect}"
      expect(brand.fetch('textContained') || brand.fetch('clipped')).to be(true),
                                                                        "font=#{font_size} geometry=#{geometry.inspect}"
      expect(geometry.fetch('personalInfo')).to include('cardWithinHorizontalViewport' => true),
                                                "font=#{font_size} geometry=#{geometry.inspect}"
      expect(geometry.fetch('personalInfo').fetch('rows')).to all(
        include('textContained' => true)
      ), "font=#{font_size} geometry=#{geometry.inspect}"
      expect(geometry.fetch('summary')).to include('textContained' => true),
                                           "font=#{font_size} geometry=#{geometry.inspect}"
      expect(geometry.fetch('summary').fetch('badges')).to all(
        include('textContained' => true)
      ), "font=#{font_size} geometry=#{geometry.inspect}"
      expect(geometry.fetch('hero')).to include(
        'titleContained' => true,
        'emailContained' => true
      ), "font=#{font_size} geometry=#{geometry.inspect}"
      expect(geometry.fetch('railLabels')).to all(
        include('withinLink' => true, 'withinRail' => true, 'contained' => true)
      ), "font=#{font_size} geometry=#{geometry.inspect}"
      expect(geometry.fetch('warning')).to include(
        'textContained' => true,
        'dismissWithinAlert' => true,
        'descriptionBelowDismiss' => true,
        'textRangesAvoidDismiss' => true
      ), "font=#{font_size} geometry=#{geometry.inspect}"
      expect(geometry.fetch('pageOverflow')).to be <= 1, "font=#{font_size} geometry=#{geometry.inspect}"
    end

    page.current_window.resize_to(1280, 900)
    page.execute_script("document.documentElement.style.fontSize = '16px'")
    desktop_warning = page.evaluate_script(desktop_warning_geometry_script)
    expect(desktop_warning).to include(
      'paddingInlineStart' => '32px',
      'textRangesAvoidDismiss' => true
    ), "desktop warning geometry=#{desktop_warning.inspect}"
  end

  it 'keeps profile labels readable in a deliberate narrow layout at enlarged text', :aggregate_failures do
    page.execute_script('document.documentElement.style.fontSize = "32px"')
    wait_for_enlarged_tab_font
    geometry = page.evaluate_script(profile_tab_geometry_script)

    expect(geometry.fetch('pageOverflow')).to be <= 1, "geometry=#{geometry.inspect}"
    expect(geometry.fetch('tabList')).to include('display' => 'flex', 'flexWrap' => 'wrap'),
                                         "geometry=#{geometry.inspect}"
    expect(geometry.fetch('rows').map { |row| row.fetch('count') }).to eq([2, 2]),
                                                                       "geometry=#{geometry.inspect}"
    expect(geometry.fetch('tabs')).to all(
      include(
        'labelContained' => true,
        'labelLineCount' => be <= 2,
        'width' => be >= 96
      )
    ), "geometry=#{geometry.inspect}"

    page.current_window.resize_to(1280, 900)
    page.execute_script('document.documentElement.style.fontSize = "16px"')
    visit profile_path
    expect(page).to have_css('[data-testid="profile-section-tab"]', count: 4)
    desktop_geometry = page.evaluate_script(profile_tab_geometry_script)
    expect(desktop_geometry.fetch('rows').map { |row| row.fetch('count') }).to eq([4]),
                                                                               "geometry=#{desktop_geometry.inspect}"
    expect(desktop_geometry.fetch('pageOverflow')).to be <= 1,
                                                      "geometry=#{desktop_geometry.inspect}"
  end

  private

  def wait_for_enlarged_tab_font
    page.evaluate_async_script(<<~JAVASCRIPT)
      const done = arguments[0];
      const ready = () => Array.from(document.querySelectorAll('[data-testid="profile-section-tab"]'))
        .every((tab) => getComputedStyle(tab).fontSize === '24px');
      const check = () => ready() ? done(true) : requestAnimationFrame(check);
      check();
    JAVASCRIPT
  end

  EnlargedTextGeometry.const_set(:SHELL_GEOMETRY_SCRIPT, <<~JAVASCRIPT)
    (() => {
      const viewport = { width: document.documentElement.clientWidth, height: document.documentElement.clientHeight };
      const rect = (element) => {
        const value = element?.getBoundingClientRect();
        return value && { left: value.left, right: value.right, top: value.top, bottom: value.bottom,
          width: value.width, height: value.height };
      };
      const textRects = (element) => {
        if (!element) return [];
        const range = document.createRange();
        range.selectNodeContents(element);
        return Array.from(range.getClientRects()).filter((value) => value.width > 0 && value.height > 0)
          .map((value) => ({ left: value.left, right: value.right, top: value.top, bottom: value.bottom }));
      };
      const within = (inner, outer) => inner && outer && inner.left >= outer.left - 1 &&
        inner.right <= outer.right + 1 && inner.top >= outer.top - 1 && inner.bottom <= outer.bottom + 1;
      const viewportWithin = (value) => value && value.left >= -1 && value.right <= viewport.width + 1 &&
        value.top >= -1 && value.bottom <= viewport.height + 1;
      const control = (selector) => {
        const element = document.querySelector(selector);
        const bounds = rect(element);
        const labels = textRects(element);
        const style = element && getComputedStyle(element);
        return {
          selector, bounds, text: element?.textContent.trim(),
          fontSize: element && getComputedStyle(element).fontSize,
          clipped: Boolean(bounds && labels.some((label) => !within(label, bounds)) &&
            style.textOverflow === 'ellipsis' && ['hidden', 'clip'].includes(style.overflowX)),
          withinViewport: viewportWithin(bounds),
          textContained: Boolean(bounds && bounds.width > 0 && bounds.height > 0 &&
            (labels.length === 0 || labels.every((label) => within(label, bounds))))
        };
      };
      const rail = document.querySelector('[data-testid="mobile-rail"]');
      const railRect = rect(rail);
      const railLabels = Array.from(document.querySelectorAll('[data-testid="mobile-rail"] a')).map((link) => {
        const linkRect = rect(link);
        const label = link.querySelector('span');
        const labelRect = rect(label);
        const labels = textRects(label);
        return {
          text: label?.textContent.trim(), linkRect, labelRect,
          fontSize: label && getComputedStyle(label).fontSize,
          withinLink: Boolean(labels.length > 0 && labels.every((value) => within(value, linkRect))),
          withinRail: Boolean(labels.length > 0 && labels.every((value) => within(value, railRect))),
          contained: Boolean(label && label.scrollWidth <= label.clientWidth + 1 &&
            label.scrollHeight <= label.clientHeight + 1)
        };
      });
      const alert = document.querySelector('[data-testid="notice-stack"] [role="alert"]');
      const warningText = alert?.querySelector('div.text-sm');
      const dismiss = alert?.querySelector('button');
      const alertRect = rect(alert);
      const warningRect = rect(warningText);
      const warningLabels = textRects(warningText);
      const dismissRect = rect(dismiss);
      const personalInfoCard = document.querySelector('[data-testid="profile-personal-info-card"]');
      const personalInfoCardRect = rect(personalInfoCard);
      const personalInfo = {
        cardRect: personalInfoCardRect,
        cardWithinHorizontalViewport: Boolean(personalInfoCardRect && personalInfoCardRect.left >= -1 &&
          personalInfoCardRect.right <= viewport.width + 1),
        rows: Array.from(personalInfoCard?.querySelectorAll('dd') || []).map((row) => ({
          text: row.textContent.trim(), bounds: rect(row),
          textContained: Boolean(textRects(row).length > 0 && textRects(row).every((value) =>
            within(value, personalInfoCardRect)))
        }))
      };
      const summary = document.querySelector('[data-profile-section-summary="profile"]');
      const summaryRect = rect(summary);
      const summaryLabels = textRects(summary);
      const summaryBadges = Array.from(summary?.querySelectorAll(':scope > span') || []).map((badge) => {
        const badgeRect = rect(badge);
        const labels = textRects(badge);
        return {
          text: badge.textContent.trim(), bounds: badgeRect,
          textContained: Boolean(badgeRect && labels.length > 0 &&
            labels.every((value) => within(value, badgeRect)))
        };
      });
      const hero = document.querySelector('[data-testid="profile-hero"]');
      const heroRect = rect(hero);
      const heroTitle = hero?.querySelector('h1');
      const heroEmail = hero?.querySelector('p.break-all');
      const heroContained = (element) => {
        const elementRect = rect(element);
        const labels = textRects(element);
        return Boolean(elementRect && labels.length > 0 && labels.every((value) => within(value, heroRect)) &&
          within(elementRect, heroRect));
      };
      const overflowSources = Array.from(document.querySelectorAll('body *')).map((element) => {
        const bounds = element.getBoundingClientRect();
        return {
          tag: element.tagName, className: element.className?.toString(),
          text: element.childElementCount === 0 ? element.textContent.trim().slice(0, 40) : '',
          right: bounds.right, left: bounds.left, width: bounds.width
        };
      }).filter((value) => value.right > viewport.width + 1 || value.left < -1)
        .sort((left, right) => right.right - left.right).slice(0, 5);
      const scrollSources = Array.from(document.querySelectorAll('body *')).map((element) => ({
        tag: element.tagName, className: element.className?.toString(),
        scrollWidth: element.scrollWidth, clientWidth: element.clientWidth
      })).filter((value) => value.scrollWidth > value.clientWidth + 1)
        .sort((left, right) =>
          (right.scrollWidth - right.clientWidth) - (left.scrollWidth - left.clientWidth)
        ).slice(0, 5);

      return {
        menu: control('button[aria-label="Open menu"]'),
        search: control('header.app-mobile-top-bar button[aria-label="Open global search"]'),
        brand: control('.nav__brand-link'),
        railLabels,
        warning: {
          alertRect, warningRect, text: warningText?.textContent.trim(),
          fontSize: warningText && getComputedStyle(warningText).fontSize,
          textContained: Boolean(warningRect && warningLabels.length > 0 &&
            warningLabels.every((value) => within(value, warningRect)) && within(warningRect, alertRect)),
          dismissWithinAlert: Boolean(dismiss && within(dismissRect, alertRect)),
          dismissRect,
          descriptionBelowDismiss: Boolean(warningRect && dismissRect &&
            warningRect.top >= dismissRect.bottom - 1),
          textRangesAvoidDismiss: Boolean(dismissRect && warningLabels.length > 0 &&
            warningLabels.every((value) => value.right <= dismissRect.left + 1 ||
              value.left >= dismissRect.right - 1 || value.bottom <= dismissRect.top + 1 ||
              value.top >= dismissRect.bottom - 1))
        },
        personalInfo,
        summary: {
          bounds: summaryRect,
          text: summary?.textContent.trim(),
          badges: summaryBadges,
          textContained: Boolean(summaryRect && summaryLabels.length > 0 &&
            summaryLabels.every((value) => within(value, summaryRect)))
        },
        hero: {
          bounds: heroRect,
          titleBounds: rect(heroTitle),
          titleText: heroTitle?.textContent.trim(),
          titleContained: heroContained(heroTitle),
          emailContained: heroContained(heroEmail)
        },
        pageOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - viewport.width,
        overflowSources, scrollSources,
        documentWidth: document.documentElement.scrollWidth, bodyWidth: document.body.scrollWidth
      };
    })()
  JAVASCRIPT

  def shell_geometry_script
    EnlargedTextGeometry::SHELL_GEOMETRY_SCRIPT
  end

  EnlargedTextGeometry.const_set(:PROFILE_TAB_GEOMETRY_SCRIPT, <<~JAVASCRIPT)
    (() => {
      const tabList = document.querySelector('[role="tablist"]');
      const listRect = tabList.getBoundingClientRect();
      const tabs = Array.from(tabList.querySelectorAll('[data-testid="profile-section-tab"]'));
      const textRects = (element) => {
        const range = document.createRange();
        range.selectNodeContents(element);
        return Array.from(range.getClientRects()).filter((value) => value.width > 0 && value.height > 0);
      };
      const rows = tabs.reduce((result, tab) => {
        const top = Math.round(tab.getBoundingClientRect().top);
        const row = result.find((value) => Math.abs(value.top - top) <= 1);
        if (row) row.count += 1;
        else result.push({ top, count: 1 });
        return result;
      }, []);

      return {
        tabList: {
          display: getComputedStyle(tabList).display,
          flexWrap: getComputedStyle(tabList).flexWrap,
          flexDirection: getComputedStyle(tabList).flexDirection,
          gap: getComputedStyle(tabList).gap,
          width: listRect.width,
          height: listRect.height
        },
        rows,
          tabs: tabs.map((tab) => {
            const bounds = tab.getBoundingClientRect();
            const label = tab.querySelector('span') || tab;
            const labels = textRects(label);
            const style = getComputedStyle(tab);
          return {
            label: tab.textContent.trim(), width: bounds.width, height: bounds.height,
            display: style.display, flexBasis: style.flexBasis, minWidth: style.minWidth,
            fontSize: style.fontSize,
            labelLineCount: labels.length,
            labelContained: labels.length > 0 && labels.every((value) =>
              value.left >= bounds.left - 1 && value.right <= bounds.right + 1 &&
              value.top >= bounds.top - 1 && value.bottom <= bounds.bottom + 1
            )
          };
        }),
        pageOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) -
          document.documentElement.clientWidth
      };
    })()
  JAVASCRIPT

  def profile_tab_geometry_script
    EnlargedTextGeometry::PROFILE_TAB_GEOMETRY_SCRIPT
  end

  EnlargedTextGeometry.const_set(:DESKTOP_WARNING_GEOMETRY_SCRIPT, <<~JAVASCRIPT)
    (() => {
      const alert = document.querySelector('[data-testid="notice-stack"] [role="alert"]');
      const description = alert?.querySelector('div.text-sm');
      const dismiss = alert?.querySelector('button');
      const range = document.createRange();
      range.selectNodeContents(description);
      const textRanges = Array.from(range.getClientRects());
      const dismissRect = dismiss?.getBoundingClientRect();
      return {
        paddingInlineStart: description && getComputedStyle(description).paddingInlineStart,
        textRangesAvoidDismiss: Boolean(dismissRect && textRanges.length > 0 && textRanges.every((value) =>
          value.right <= dismissRect.left + 1 || value.left >= dismissRect.right - 1 ||
          value.bottom <= dismissRect.top + 1 || value.top >= dismissRect.bottom - 1))
      };
    })()
  JAVASCRIPT

  def desktop_warning_geometry_script
    EnlargedTextGeometry::DESKTOP_WARNING_GEOMETRY_SCRIPT
  end
end
