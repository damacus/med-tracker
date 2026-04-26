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

    def import(release_dir, progress_callback: nil)
      dir = Pathname.new(release_dir)
      ampp_file = glob_one(dir, 'f_ampp2_3*.xml')
      gtin_file = find_gtin_file(dir)

      counts = build_counts(ampp_file, gtin_file)
      emit_initial_progress(progress_callback, counts)

      names = parse_ampp_names(ampp_file, counts:, progress_callback: progress_callback)
      emit_gtin_start_progress(progress_callback, counts)

      import_gtins(gtin_file, names, counts:, progress_callback: progress_callback)
    end

    private

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
      system('unzip', '-o', zip_path.to_s, 'f_gtin2_0*.xml', '-d', dest.to_s,
             exception: true)
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
        names[appid] = name
      end
    ensure
      emit_progress(counts, progress_callback, force: true, message: ampp_progress_message(counts))
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

      display = names[amppid]
      doc.css('GTINDATA').each do |gtin_data|
        import_gtin_data(gtin_data, amppid:, display:, today:, counts:)
        emit_progress(counts, progress_callback, message: gtin_progress_message(counts))
      end
    end

    def import_gtin_data(gtin_data, amppid:, display:, today:, counts:)
      mark_gtin_processed(counts)
      gtin = node_text(gtin_data, 'GTIN')
      return increment(counts, :skipped_invalid) if gtin.blank?
      return increment(counts, :skipped_expired) if expired?(gtin_data, today)
      return increment(counts, :skipped_missing_name) if display.blank?

      outcome = persist(
        gtin: NhsDmdBarcode.normalize_gtin(gtin),
        code: amppid,
        display: display,
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

    def count_gtin_records(path) = count_records(path, 'GTINDATA')

    def count_ampp_records(path) = count_records(path, 'AMPP')

    def count_records(path, node_name)
      Nokogiri::XML::Reader(File.open(path)).count do |node|
        node.name == node_name && node.node_type == Nokogiri::XML::Reader::TYPE_ELEMENT
      end
    end
  end
end
