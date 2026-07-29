# frozen_string_literal: true

require 'nokogiri'

module NhsDmd
  class ReleaseImport
    include ReleaseImportCounts
    include ReleaseImportProgress

    RESULT_KEYS = %i[
      created_count
      updated_count
      unchanged_count
      skipped_expired_count
      skipped_missing_name_count
      skipped_invalid_count
    ].freeze

    Result = Struct.new(*RESULT_KEYS, keyword_init: true) do
      def imported_count = created_count + updated_count
      def skipped_count = skipped_expired_count + skipped_missing_name_count + skipped_invalid_count
    end

    def initialize(extractor: ReleaseArchiveExtractor.new)
      @extractor = extractor
    end

    def import(release_dir, progress_callback: nil)
      dir = Pathname.new(release_dir)
      ampp_file = glob_one(dir, 'f_ampp2_3*.xml')
      gtin_file = find_gtin_file(dir)

      counts = build_counts(ampp_file, gtin_file)
      emit_initial_progress(progress_callback, counts)

      names = parse_ampp_names(ampp_file, counts:, progress_callback: progress_callback)
      import_ampp_relationships(names)
      import_supplementary_metadata(dir)
      emit_gtin_start_progress(progress_callback, counts)

      import_gtins(gtin_file, names, counts:, progress_callback: progress_callback)
    end

    private

    attr_reader :extractor

    def glob_one(dir, pattern)
      matches = Dir.glob(dir.join(pattern))
      raise ArgumentError, "No file matching #{pattern} in #{dir}" if matches.empty?

      matches.first
    end

    def find_gtin_file(dir)
      existing = Dir.glob(dir.join('f_gtin2_0*.xml'))
      return existing.first if existing.any?

      zip = Dir.glob(dir.join('*GTIN.zip')).first
      raise ArgumentError, "No GTIN XML or ZIP found in #{dir}" unless zip

      extract_dir = Dir.mktmpdir('dmd-gtin')
      extract_gtin_xml(zip, extract_dir)
      Dir.glob(File.join(extract_dir, 'f_gtin2_0*.xml')).first
    end

    def extract_gtin_xml(zip_path, dest)
      extractor.extract(zip_path, dest, pattern: 'f_gtin2_0*.xml')
    end

    def parse_ampp_names(path, counts:, progress_callback:)
      each_ampp_doc(path).with_object({}) do |doc, names|
        appid = node_text(doc, 'APPID')
        name = node_text(doc, 'NM')
        track_ampp_progress(counts, progress_callback)
        if appid.blank? || name.blank?
          counts[:ampp_skipped] += 1
          next
        end

        counts[:ampp_named] += 1
        names[appid] = { name: name, amp_code: node_text(doc, 'APID') }
      end
    ensure
      emit_progress(counts, progress_callback, force: true, message: ampp_progress_message(counts))
    end

    def import_ampp_relationships(names)
      timestamp = Time.current
      relationships = names.filter_map do |ampp_code, attributes|
        amp_code = attributes[:amp_code]
        { ampp_code: ampp_code, amp_code: amp_code, created_at: timestamp, updated_at: timestamp } if amp_code.present?
      end

      NhsDmdAmppRelationship.transaction do
        NhsDmdAmppRelationship.delete_all
        NhsDmdAmppRelationship.create!(relationships)
      end
    end

    def import_gtins(path, names, counts:, progress_callback:)
      today = Time.zone.today

      each_ampp_doc(path) do |doc|
        import_ampp_gtins(doc, names:, today:, counts:, progress_callback:)
      end

      emit_progress(counts, progress_callback, force: true, message: gtin_progress_message(counts))

      build_result(counts)
    end

    def each_ampp_doc(path)
      return enum_for(__method__, path) unless block_given?

      Nokogiri::XML::Reader(File.open(path)).each do |node|
        next unless ampp_element?(node)

        yield Nokogiri::XML(node.outer_xml)
      end
    end

    def ampp_element?(node)
      node.name == 'AMPP' && node.node_type == Nokogiri::XML::Reader::TYPE_ELEMENT
    end

    def import_ampp_gtins(doc, names:, today:, counts:, progress_callback:)
      amppid = node_text(doc, 'AMPPID')
      if amppid.blank?
        process_unmatched_gtins(doc, counts, progress_callback)
        return
      end

      ampp = names[amppid]
      display = ampp&.fetch(:name)
      doc.css('GTINDATA').each do |gtin_data|
        import_gtin_data(gtin_data, amppid:, amp_code: ampp&.fetch(:amp_code), display:, today:, counts:)
        emit_progress(counts, progress_callback, message: gtin_progress_message(counts))
      end
    end

    def import_gtin_data(gtin_data, amppid:, amp_code:, display:, today:, counts:)
      mark_gtin_processed(counts)
      gtin = node_text(gtin_data, 'GTIN')
      return increment(counts, :skipped_invalid) if gtin.blank?
      return increment(counts, :skipped_expired) if expired?(gtin_data, today)
      return increment(counts, :skipped_missing_name) if display.blank?

      stripped = display.sub(/\s*\([^)]*\)\z/, '').strip
      outcome = persist(
        gtin: NhsDmdBarcode.normalize_gtin(gtin),
        code: amppid,
        amp_code: amp_code,
        display: display,
        vmp_name: stripped == display ? nil : stripped,
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )
      increment(counts, outcome)
    end

    def increment(counts, key)
      counts[key] += 1
      nil
    end

    def expired?(gtin_data, today)
      end_date = node_text(gtin_data, 'ENDDT')
      end_date.present? && Date.parse(end_date) <= today
    end

    def node_text(doc, selector)
      doc.at(selector)&.text
    end

    def persist(attrs)
      record = NhsDmdBarcode.find_or_initialize_by(gtin: attrs[:gtin])
      record.assign_attributes(attrs)

      if record.new_record?
        record.save!
        :created
      elsif record.changed?
        record.save!
        :updated
      else
        :unchanged
      end
    end

    def import_supplementary_metadata(dir)
      files = supplementary_files(dir)
      return unless files.values.all?

      metadata = stage_supplementary_metadata(files)
      return unless metadata

      ActiveRecord::Base.transaction do
        import_trade_family_groups(metadata[:groups])
        import_trade_families(metadata[:families])
        import_amp_trade_families(metadata[:memberships])
        record_supplementary_release(metadata[:released_on])
      end
      expire_barcode_lookup_cache
    rescue Nokogiri::XML::SyntaxError, IOError, SystemCallError => error
      Rails.logger.warn("Unable to import dm+d supplementary metadata: #{error.message}")
    end

    def supplementary_files(dir)
      {
        groups: optional_glob_one(dir, 'f_trade_family_group2_0*.xml'),
        families: optional_glob_one(dir, 'f_trade_family2_0*.xml'),
        memberships: optional_glob_one(dir, 'f_amp_trade_family2_0*.xml')
      }
    end

    def optional_glob_one(dir, pattern)
      matches = Dir.glob(dir.join(pattern))
      return matches.first if matches.one?

      Rails.logger.warn("Unable to import dm+d supplementary metadata: expected one #{pattern} file, found #{matches.length}") if matches.many?
      nil
    end

    def expire_barcode_lookup_cache
      NhsDmd::BarcodeLookup.expire_all
    rescue StandardError, NotImplementedError => error
      Rails.logger.warn("Unable to expire dm+d barcode lookup cache: #{error.class}: #{error.message}")
    end

    def stage_supplementary_metadata(files)
      groups = parse_trade_family_groups(files[:groups])
      families = parse_trade_families(files[:families])
      memberships = parse_amp_trade_families(files[:memberships])
      return if groups.empty? || families.empty?
      return unless valid_supplementary_metadata?(groups, families, memberships)

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

    def valid_supplementary_metadata?(groups, families, memberships)
      group_codes = groups.pluck(:code)
      family_codes = families.pluck(:code)

      group_codes.uniq.length == group_codes.length &&
        family_codes.uniq.length == family_codes.length &&
        memberships.map { |membership| membership[:amp_code] }.uniq.length == memberships.length &&
        families.all? { |family| family[:group_code].blank? || group_codes.include?(family[:group_code]) } &&
        memberships.all? { |membership| family_codes.include?(membership[:trade_family_code]) }
    end

    def import_trade_family_groups(groups)
      groups.each do |group|
        NhsDmdTradeFamilyGroup.upsert(
          group.merge(updated_at: Time.current, created_at: Time.current),
          unique_by: :code
        )
      end
    end

    def import_trade_families(families)
      families.each do |family|
        group = NhsDmdTradeFamilyGroup.find_by(code: family[:group_code])
        NhsDmdTradeFamily.upsert(
          family.slice(:code, :name).merge(
            trade_family_group_id: group&.id,
            updated_at: Time.current,
            created_at: Time.current
          ),
          unique_by: :code
        )
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
      return unless released_on

      NhsDmdSupplementaryRelease.find_or_create_by!(released_on: released_on)
    end

    def supplementary_release_date(files) = files.filter_map { |file| release_date_from_filename(file) }.max

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

    def count_gtin_records(path) = count_records(path, 'GTINDATA')

    def count_ampp_records(path) = count_records(path, 'AMPP')

    def count_records(path, node_name)
      Nokogiri::XML::Reader(File.open(path)).count do |node|
        node.name == node_name && node.node_type == Nokogiri::XML::Reader::TYPE_ELEMENT
      end
    end
  end
end
