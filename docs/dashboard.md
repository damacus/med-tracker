# Feature Status Dashboard

<div id="dashboard-container">
  <div id="loading" style="text-align: center; padding: 40px;">
    <p>Loading feature data...</p>
  </div>
</div>

<style>
  /* Dashboard Styling */
  #dashboard-container {
    max-width: 100%;
    margin: 20px 0;
  }

  .dashboard-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 20px;
    margin-bottom: 30px;
  }

  .stat-card {
    background: var(--md-code-bg-color);
    border: 1px solid var(--md-default-fg-color--lightest);
    border-radius: 8px;
    padding: 20px;
    text-align: center;
    transition: transform 0.2s, box-shadow 0.2s;
  }

  .stat-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
  }

  .stat-number {
    font-size: 2.5em;
    font-weight: bold;
    margin: 10px 0;
  }

  .stat-label {
    font-size: 0.9em;
    color: var(--md-default-fg-color--light);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .stat-pass { color: #4caf50; }
  .stat-fail { color: #f44336; }
  .stat-total { color: #2196f3; }
  .stat-rate { color: #ff9800; }

  /* Controls Section */
  .dashboard-controls {
    background: var(--md-code-bg-color);
    border: 1px solid var(--md-default-fg-color--lightest);
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 20px;
  }

  .controls-row {
    display: flex;
    flex-wrap: wrap;
    gap: 15px;
    margin-bottom: 15px;
    align-items: center;
  }

  .control-group {
    flex: 1;
    min-width: 200px;
  }

  .control-group label {
    display: block;
    font-size: 0.85em;
    font-weight: 600;
    margin-bottom: 5px;
    color: var(--md-default-fg-color--light);
  }

  .control-group input,
  .control-group select {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--md-default-fg-color--lightest);
    border-radius: 4px;
    background: var(--md-default-bg-color);
    color: var(--md-default-fg-color);
    font-size: 0.9em;
  }

  .control-group input:focus,
  .control-group select:focus {
    outline: none;
    border-color: var(--md-primary-fg-color);
    box-shadow: 0 0 0 2px var(--md-primary-fg-color--light);
  }

  .btn-reset {
    padding: 8px 16px;
    background: var(--md-primary-fg-color);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9em;
    font-weight: 500;
    transition: background 0.2s;
  }

  .btn-reset:hover {
    background: var(--md-accent-fg-color);
  }

  /* Table Styling */
  .features-table-wrapper {
    overflow-x: auto;
    background: var(--md-code-bg-color);
    border: 1px solid var(--md-default-fg-color--lightest);
    border-radius: 8px;
  }

  .features-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9em;
  }

  .features-table thead {
    background: var(--md-default-fg-color--lightest);
  }

  .features-table th {
    padding: 12px 16px;
    text-align: left;
    font-weight: 600;
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
    position: relative;
  }

  .features-table th:hover {
    background: var(--md-default-fg-color--lighter);
  }

  .features-table th.sortable::after {
    content: ' ⇅';
    opacity: 0.3;
    font-size: 0.8em;
  }

  .features-table th.sorted-asc::after {
    content: ' ↑';
    opacity: 1;
  }

  .features-table th.sorted-desc::after {
    content: ' ↓';
    opacity: 1;
  }

  .features-table td {
    padding: 12px 16px;
    border-top: 1px solid var(--md-default-fg-color--lightest);
  }

  .features-table tbody tr:hover {
    background: var(--md-default-fg-color--lightest);
  }

  .status-badge {
    display: inline-block;
    padding: 4px 12px;
    border-radius: 12px;
    font-size: 0.85em;
    font-weight: 600;
    text-transform: uppercase;
  }

  .status-pass {
    background: #e8f5e9;
    color: #2e7d32;
  }

  .status-fail {
    background: #ffebee;
    color: #c62828;
  }

  .category-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.8em;
    background: var(--md-default-fg-color--lightest);
    color: var(--md-default-fg-color);
  }

  .no-results {
    text-align: center;
    padding: 40px;
    color: var(--md-default-fg-color--light);
    font-style: italic;
  }

  /* Area breakdown section */
  .area-breakdown {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 15px;
    margin-top: 30px;
  }

  .area-card {
    background: var(--md-code-bg-color);
    border: 1px solid var(--md-default-fg-color--lightest);
    border-radius: 8px;
    padding: 15px;
  }

  .area-card h4 {
    margin: 0 0 10px 0;
    font-size: 1em;
    color: var(--md-primary-fg-color);
  }

  .area-stats {
    display: flex;
    justify-content: space-between;
    font-size: 0.9em;
  }

  .area-progress {
    width: 100%;
    height: 8px;
    background: var(--md-default-fg-color--lightest);
    border-radius: 4px;
    margin-top: 10px;
    overflow: hidden;
  }

  .area-progress-bar {
    height: 100%;
    background: linear-gradient(90deg, #4caf50, #8bc34a);
    transition: width 0.3s ease;
  }

  /* Responsive adjustments */
  @media (max-width: 768px) {
    .dashboard-stats {
      grid-template-columns: repeat(2, 1fr);
    }
    
    .features-table {
      font-size: 0.85em;
    }
    
    .features-table th,
    .features-table td {
      padding: 8px 10px;
    }
  }

  @media (max-width: 480px) {
    .dashboard-stats {
      grid-template-columns: 1fr;
    }

    .controls-row {
      flex-direction: column;
    }

    .control-group {
      width: 100%;
    }
  }
</style>

<script>
(function() {
  // State management
  let allFeatures = [];
  let filteredFeatures = [];
  let sortColumn = 'id';
  let sortDirection = 'asc';

  // DOM Elements (will be set after load)
  let statsContainer, tableContainer, controlsContainer;

  // Load all feature files
  async function loadFeatures() {
    const featureFiles = [
      'accessibility.json',
      'admin.json',
      'audit.json',
      'authentication.json',
      'authorization.json',
      'carer_relationships.json',
      'dashboard.json',
      'dosages.json',
      'dose_tracking.json',
      'e2e.json',
      'i18n.json',
      'invitations.json',
      'medicine_lookup.json',
      'medicines.json',
      'navigation.json',
      'observability.json',
      'people.json',
      'performance.json',
      'person_medicines.json',
      'prescriptions.json',
      'profile.json',
      'pwa.json',
      'security.json',
      'ui.json',
      'ui_improvements.json'
    ];

    const features = [];
    
    for (const file of featureFiles) {
      try {
        const response = await fetch(`../features/${file}`);
        if (response.ok) {
          const data = await response.json();
          features.push(...data);
        }
      } catch (error) {
        console.warn(`Failed to load ${file}:`, error);
      }
    }

    return features;
  }

  // Calculate statistics
  function calculateStats(features) {
    const total = features.length;
    const passed = features.filter(f => f.passes === true).length;
    const failed = total - passed;
    const passRate = total > 0 ? ((passed / total) * 100).toFixed(1) : 0;

    // Area breakdown
    const areaStats = {};
    features.forEach(f => {
      if (!areaStats[f.area]) {
        areaStats[f.area] = { total: 0, passed: 0 };
      }
      areaStats[f.area].total++;
      if (f.passes === true) {
        areaStats[f.area].passed++;
      }
    });

    // Category breakdown
    const categoryStats = {};
    features.forEach(f => {
      if (!categoryStats[f.category]) {
        categoryStats[f.category] = { total: 0, passed: 0 };
      }
      categoryStats[f.category].total++;
      if (f.passes === true) {
        categoryStats[f.category].passed++;
      }
    });

    return { total, passed, failed, passRate, areaStats, categoryStats };
  }

  // Render statistics cards
  function renderStats(stats) {
    return `
      <div class="dashboard-stats">
        <div class="stat-card">
          <div class="stat-label">Total Features</div>
          <div class="stat-number stat-total">${stats.total}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Passing</div>
          <div class="stat-number stat-pass">${stats.passed}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Failing</div>
          <div class="stat-number stat-fail">${stats.failed}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Pass Rate</div>
          <div class="stat-number stat-rate">${stats.passRate}%</div>
        </div>
      </div>
    `;
  }

  // Render area breakdown
  function renderAreaBreakdown(areaStats) {
    const areas = Object.keys(areaStats).sort();
    return `
      <h3>Features by Area</h3>
      <div class="area-breakdown">
        ${areas.map(area => {
          const stats = areaStats[area];
          const passRate = ((stats.passed / stats.total) * 100).toFixed(0);
          return `
            <div class="area-card">
              <h4>${area}</h4>
              <div class="area-stats">
                <span>${stats.passed}/${stats.total} passing</span>
                <span>${passRate}%</span>
              </div>
              <div class="area-progress">
                <div class="area-progress-bar" style="width: ${passRate}%"></div>
              </div>
            </div>
          `;
        }).join('')}
      </div>
    `;
  }

  // Render controls
  function renderControls() {
    const areas = [...new Set(allFeatures.map(f => f.area))].sort();
    const categories = [...new Set(allFeatures.map(f => f.category))].sort();

    return `
      <div class="dashboard-controls">
        <div class="controls-row">
          <div class="control-group">
            <label for="search-input">Search</label>
            <input 
              type="text" 
              id="search-input" 
              placeholder="Search by ID or description..."
              aria-label="Search features"
            />
          </div>
          <div class="control-group">
            <label for="area-filter">Area</label>
            <select id="area-filter" aria-label="Filter by area">
              <option value="">All Areas</option>
              ${areas.map(area => `<option value="${area}">${area}</option>`).join('')}
            </select>
          </div>
          <div class="control-group">
            <label for="category-filter">Category</label>
            <select id="category-filter" aria-label="Filter by category">
              <option value="">All Categories</option>
              ${categories.map(cat => `<option value="${cat}">${cat}</option>`).join('')}
            </select>
          </div>
          <div class="control-group">
            <label for="status-filter">Status</label>
            <select id="status-filter" aria-label="Filter by status">
              <option value="">All</option>
              <option value="pass">Passing</option>
              <option value="fail">Failing</option>
            </select>
          </div>
          <div class="control-group" style="display: flex; align-items: flex-end;">
            <button class="btn-reset" id="reset-filters" aria-label="Reset all filters">
              Reset Filters
            </button>
          </div>
        </div>
      </div>
    `;
  }

  // Render table
  function renderTable(features) {
    if (features.length === 0) {
      return '<div class="no-results">No features match your filters.</div>';
    }

    return `
      <div class="features-table-wrapper">
        <table class="features-table" role="table">
          <thead>
            <tr>
              <th class="sortable ${sortColumn === 'id' ? 'sorted-' + sortDirection : ''}" 
                  data-column="id" 
                  role="columnheader"
                  aria-sort="${sortColumn === 'id' ? sortDirection + 'ending' : 'none'}">
                ID
              </th>
              <th class="sortable ${sortColumn === 'area' ? 'sorted-' + sortDirection : ''}" 
                  data-column="area"
                  role="columnheader"
                  aria-sort="${sortColumn === 'area' ? sortDirection + 'ending' : 'none'}">
                Area
              </th>
              <th class="sortable ${sortColumn === 'category' ? 'sorted-' + sortDirection : ''}" 
                  data-column="category"
                  role="columnheader"
                  aria-sort="${sortColumn === 'category' ? sortDirection + 'ending' : 'none'}">
                Category
              </th>
              <th role="columnheader">Description</th>
              <th class="sortable ${sortColumn === 'passes' ? 'sorted-' + sortDirection : ''}" 
                  data-column="passes"
                  role="columnheader"
                  aria-sort="${sortColumn === 'passes' ? sortDirection + 'ending' : 'none'}">
                Status
              </th>
            </tr>
          </thead>
          <tbody>
            ${features.map(f => `
              <tr>
                <td><strong>${f.id}</strong></td>
                <td>${f.area}</td>
                <td><span class="category-badge">${f.category}</span></td>
                <td>${f.description}</td>
                <td>
                  <span class="status-badge ${f.passes ? 'status-pass' : 'status-fail'}" 
                        role="status"
                        aria-label="${f.passes ? 'Passing' : 'Failing'}">
                    ${f.passes ? '✓ Pass' : '✗ Fail'}
                  </span>
                </td>
              </tr>
            `).join('')}
          </tbody>
        </table>
      </div>
    `;
  }

  // Apply filters
  function applyFilters() {
    const searchTerm = document.getElementById('search-input').value.toLowerCase();
    const areaFilter = document.getElementById('area-filter').value;
    const categoryFilter = document.getElementById('category-filter').value;
    const statusFilter = document.getElementById('status-filter').value;

    filteredFeatures = allFeatures.filter(f => {
      // Search filter
      if (searchTerm && 
          !f.id.toLowerCase().includes(searchTerm) && 
          !f.description.toLowerCase().includes(searchTerm)) {
        return false;
      }

      // Area filter
      if (areaFilter && f.area !== areaFilter) {
        return false;
      }

      // Category filter
      if (categoryFilter && f.category !== categoryFilter) {
        return false;
      }

      // Status filter
      if (statusFilter === 'pass' && !f.passes) {
        return false;
      }
      if (statusFilter === 'fail' && f.passes) {
        return false;
      }

      return true;
    });

    sortFeatures();
    updateTable();
  }

  // Sort features
  function sortFeatures() {
    filteredFeatures.sort((a, b) => {
      let aVal = a[sortColumn];
      let bVal = b[sortColumn];

      // Handle boolean for passes
      if (sortColumn === 'passes') {
        aVal = aVal ? 1 : 0;
        bVal = bVal ? 1 : 0;
      }

      // Handle string comparison
      if (typeof aVal === 'string') {
        aVal = aVal.toLowerCase();
        bVal = bVal.toLowerCase();
      }

      if (aVal < bVal) return sortDirection === 'asc' ? -1 : 1;
      if (aVal > bVal) return sortDirection === 'asc' ? 1 : -1;
      return 0;
    });
  }

  // Update table only
  function updateTable() {
    tableContainer.innerHTML = renderTable(filteredFeatures);
    attachTableEventListeners();
  }

  // Attach event listeners to table headers
  function attachTableEventListeners() {
    document.querySelectorAll('.features-table th.sortable').forEach(th => {
      th.addEventListener('click', function() {
        const column = this.dataset.column;
        if (sortColumn === column) {
          sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
        } else {
          sortColumn = column;
          sortDirection = 'asc';
        }
        sortFeatures();
        updateTable();
      });
    });
  }

  // Initialize dashboard
  async function initDashboard() {
    const container = document.getElementById('dashboard-container');
    
    try {
      // Load features
      allFeatures = await loadFeatures();
      filteredFeatures = [...allFeatures];

      // Calculate stats
      const stats = calculateStats(allFeatures);

      // Create containers
      container.innerHTML = `
        <div id="stats-container"></div>
        <div id="controls-container"></div>
        <div id="table-container"></div>
        <div id="area-breakdown-container"></div>
      `;

      statsContainer = document.getElementById('stats-container');
      controlsContainer = document.getElementById('controls-container');
      tableContainer = document.getElementById('table-container');
      const areaBreakdownContainer = document.getElementById('area-breakdown-container');

      // Render all sections
      statsContainer.innerHTML = renderStats(stats);
      controlsContainer.innerHTML = renderControls();
      sortFeatures();
      tableContainer.innerHTML = renderTable(filteredFeatures);
      areaBreakdownContainer.innerHTML = renderAreaBreakdown(stats.areaStats);

      // Attach event listeners
      attachTableEventListeners();

      // Filter controls
      document.getElementById('search-input').addEventListener('input', applyFilters);
      document.getElementById('area-filter').addEventListener('change', applyFilters);
      document.getElementById('category-filter').addEventListener('change', applyFilters);
      document.getElementById('status-filter').addEventListener('change', applyFilters);
      
      // Reset button
      document.getElementById('reset-filters').addEventListener('click', function() {
        document.getElementById('search-input').value = '';
        document.getElementById('area-filter').value = '';
        document.getElementById('category-filter').value = '';
        document.getElementById('status-filter').value = '';
        applyFilters();
      });

    } catch (error) {
      container.innerHTML = `
        <div class="no-results">
          <p><strong>Error loading features:</strong> ${error.message}</p>
          <p>Please ensure the feature JSON files are accessible.</p>
        </div>
      `;
    }
  }

  // Start when DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initDashboard);
  } else {
    initDashboard();
  }
})();
</script>
