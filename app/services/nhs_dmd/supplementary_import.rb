# frozen_string_literal: true

require 'nokogiri'

module NhsDmd
  class SupplementaryImport
    FILE_PATTERNS = {
      groups: 'f_trade_family_group2_0*.xml',
      families: 'f_trade_family2_0*.xml',
      memberships: 'f_amp_trade_family2_0*.xml'
    }.freeze

    def call(dir)
      files = supplementary_files(dir)
      return unless files.values.all?

      metadata = stage_metadata(files)
      return unless metadata

      persist_metadata(metadata)
      expire_barcode_lookup_cache
    rescue Nokogiri::XML::SyntaxError, IOError, SystemCallError => e
      Observability::DiagnosticEvent.emit(
        component: :nhs_dmd_supplementary,
        reason: :operation_failed,
        severity: :warn,
        error: e
      )
    end

    private

    def supplementary_files(dir)
      FILE_PATTERNS.transform_values { |pattern| optional_glob_one(dir, pattern) }
    end

    def optional_glob_one(dir, pattern)
      matches = Dir.glob(dir.join(pattern))
      return matches.first if matches.one?

      warn_about_duplicate_files(pattern, matches.length) if matches.many?
      nil
    end

    def warn_about_duplicate_files(_pattern, _count)
      Observability::DiagnosticEvent.emit(
        component: :nhs_dmd_supplementary,
        reason: :invalid_payload,
        severity: :warn
      )
    end

    def stage_metadata(files)
      groups = parse_trade_family_groups(files[:groups])
      families = parse_trade_families(files[:families])
      memberships = parse_amp_trade_families(files[:memberships])
      return if groups.empty? || families.empty?
      return unless valid_metadata?(groups, families, memberships)

      {
        groups: groups,
        families: families,
        memberships: memberships,
        released_on: supplementary_release_date(files.values)
      }
    end

    def parse_trade_family_groups(path)
      each_element(path, 'TRADE_FAMILY_GROUP').filter_map do |doc|
        code = node_text(doc, 'TFGID')
        name = node_text(doc, 'NM')
        { code: code, name: name } if code.present? && name.present?
      end
    end

    def parse_trade_families(path)
      each_element(path, 'TRADE_FAMILY').filter_map do |doc|
        code = node_text(doc, 'TFID')
        name = node_text(doc, 'NM')
        next if code.blank? || name.blank?

        { code: code, name: name, group_code: node_text(doc, 'TFGID') }
      end
    end

    def parse_amp_trade_families(path)
      each_element(path, 'AMP_TRADE_FAMILY').filter_map do |doc|
        next unless active_amp_trade_family?(doc)

        amp_code = node_text(doc, 'APID')
        trade_family_code = node_text(doc, 'TFID')
        next if amp_code.blank? || trade_family_code.blank?

        { amp_code: amp_code, trade_family_code: trade_family_code }
      end
    end

    def valid_metadata?(groups, families, memberships)
      group_codes = groups.pluck(:code)
      family_codes = families.pluck(:code)

      unique_values?(group_codes) &&
        unique_values?(family_codes) &&
        unique_values?(memberships.pluck(:amp_code)) &&
        valid_group_references?(families, group_codes) &&
        valid_family_references?(memberships, family_codes)
    end

    def unique_values?(values)
      values.uniq.length == values.length
    end

    def valid_group_references?(families, group_codes)
      families.all? { |family| family[:group_code].blank? || group_codes.include?(family[:group_code]) }
    end

    def valid_family_references?(memberships, family_codes)
      memberships.all? { |membership| family_codes.include?(membership[:trade_family_code]) }
    end

    def persist_metadata(metadata)
      ActiveRecord::Base.transaction do
        import_trade_family_groups(metadata[:groups])
        import_trade_families(metadata[:families])
        import_amp_trade_families(metadata[:memberships])
        record_supplementary_release(metadata[:released_on])
      end
    end

    def import_trade_family_groups(groups)
      groups.each do |attributes|
        group = NhsDmdTradeFamilyGroup.find_or_initialize_by(code: attributes[:code])
        group.name = attributes[:name]
        group.save! if group.new_record? || group.changed?
      end
    end

    def import_trade_families(families)
      families.each do |attributes|
        family = NhsDmdTradeFamily.find_or_initialize_by(code: attributes[:code])
        family.name = attributes[:name]
        family.trade_family_group = NhsDmdTradeFamilyGroup.find_by(code: attributes[:group_code])
        family.save! if family.new_record? || family.changed?
      end
    end

    def import_amp_trade_families(memberships)
      NhsDmdAmpTradeFamily.delete_all

      memberships.each do |membership|
        trade_family = NhsDmdTradeFamily.find_by!(code: membership[:trade_family_code])
        NhsDmdAmpTradeFamily.create!(amp_code: membership[:amp_code], trade_family: trade_family)
      end
    end

    def active_amp_trade_family?(doc)
      today = Time.zone.today
      start_date, valid_start_date = supplementary_date(doc, 'STARTDT')
      end_date, valid_end_date = supplementary_date(doc, 'ENDDT')
      return false unless valid_start_date && valid_end_date

      (start_date.nil? || start_date <= today) && (end_date.nil? || end_date > today)
    end

    def supplementary_date(doc, selector)
      node = doc.at(selector)
      return [nil, true] unless node

      [Date.parse(node.text), true]
    rescue Date::Error
      [nil, false]
    end

    def record_supplementary_release(released_on)
      NhsDmdSupplementaryRelease.find_or_create_by!(released_on: released_on) if released_on
    end

    def supplementary_release_date(files)
      files.filter_map { |file| release_date_from_filename(file) }.max
    end

    def release_date_from_filename(path)
      date = File.basename(path)[/(\d{6})(?=\.xml\z)/, 1]
      Date.strptime(date, '%d%m%y') if date
    rescue Date::Error
      nil
    end

    def each_element(path, element_name)
      return enum_for(__method__, path, element_name) unless block_given?

      Nokogiri::XML::Reader(File.open(path)).each do |node|
        next unless node.name == element_name && node.node_type == Nokogiri::XML::Reader::TYPE_ELEMENT

        yield Nokogiri::XML(node.outer_xml)
      end
    end

    def node_text(doc, selector)
      doc.at(selector)&.text
    end

    def expire_barcode_lookup_cache
      NhsDmd::BarcodeLookup.expire_all
    rescue StandardError, NotImplementedError => e
      Observability::DiagnosticEvent.emit(
        component: :nhs_dmd_supplementary,
        reason: :operation_failed,
        severity: :warn,
        error: e
      )
    end
  end
end
