// Configuration
const ES_URL = 'http://localhost:9200';
const ES_INDEX = 'kv-metrics';
const REFRESH_INTERVAL = 5000;

// Chart instances
let hitMissChart = null;
let opsChart = null;
let trendingChart = null;
let opsTimeChart = null;

// Chart colors
const COLORS = {
    hits: '#22c55e',
    misses: '#ef4444',
    gets: '#3b82f6',
    sets: '#8b5cf6',
    deletes: '#f97316',
    primary: '#6366f1',
    secondary: '#8b5cf6',
    grid: 'rgba(255, 255, 255, 0.05)',
    text: '#a0a0b0'
};

// Current trending metric
let currentTrendingMetric = 'hit_ratio';

// Initialize on load
document.addEventListener('DOMContentLoaded', () => {
    initCharts();
    setupEventListeners();
    fetchAndUpdate();
    setInterval(fetchAndUpdate, REFRESH_INTERVAL);
});

function initCharts() {
    // Hit/Miss Doughnut Chart
    const hitMissCtx = document.getElementById('hitMissChart').getContext('2d');
    hitMissChart = new Chart(hitMissCtx, {
        type: 'doughnut',
        data: {
            labels: ['Hits', 'Misses'],
            datasets: [{
                data: [0, 0],
                backgroundColor: [COLORS.hits, COLORS.misses],
                borderWidth: 0,
                cutout: '75%'
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
                legend: {
                    position: 'bottom',
                    labels: { color: COLORS.text, padding: 20 }
                }
            }
        }
    });

    // Operations Breakdown Chart
    const opsCtx = document.getElementById('opsChart').getContext('2d');
    opsChart = new Chart(opsCtx, {
        type: 'doughnut',
        data: {
            labels: ['GETs', 'SETs', 'DELETEs'],
            datasets: [{
                data: [0, 0, 0],
                backgroundColor: [COLORS.gets, COLORS.sets, COLORS.deletes],
                borderWidth: 0,
                cutout: '75%'
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
                legend: {
                    position: 'bottom',
                    labels: { color: COLORS.text, padding: 20 }
                }
            }
        }
    });

    // Trending Line Chart
    const trendingCtx = document.getElementById('trendingChart').getContext('2d');
    trendingChart = new Chart(trendingCtx, {
        type: 'line',
        data: {
            labels: [],
            datasets: [{
                label: 'Hit Ratio %',
                data: [],
                borderColor: COLORS.primary,
                backgroundColor: 'rgba(99, 102, 241, 0.1)',
                fill: true,
                tension: 0.4,
                pointRadius: 3,
                pointHoverRadius: 6
            }]
        },
        options: getLineChartOptions()
    });

    // Operations Over Time Chart
    const opsTimeCtx = document.getElementById('opsTimeChart').getContext('2d');
    opsTimeChart = new Chart(opsTimeCtx, {
        type: 'line',
        data: {
            labels: [],
            datasets: [
                {
                    label: 'GETs',
                    data: [],
                    borderColor: COLORS.gets,
                    backgroundColor: 'rgba(59, 130, 246, 0.1)',
                    fill: false,
                    tension: 0.4,
                    pointRadius: 2
                },
                {
                    label: 'SETs',
                    data: [],
                    borderColor: COLORS.sets,
                    backgroundColor: 'rgba(139, 92, 246, 0.1)',
                    fill: false,
                    tension: 0.4,
                    pointRadius: 2
                }
            ]
        },
        options: getLineChartOptions()
    });
}

function getLineChartOptions() {
    return {
        responsive: true,
        maintainAspectRatio: false,
        interaction: {
            intersect: false,
            mode: 'index'
        },
        plugins: {
            legend: {
                position: 'top',
                align: 'end',
                labels: { color: COLORS.text, usePointStyle: true }
            }
        },
        scales: {
            x: {
                grid: { color: COLORS.grid },
                ticks: { color: COLORS.text, maxTicksLimit: 10 }
            },
            y: {
                grid: { color: COLORS.grid },
                ticks: { color: COLORS.text }
            }
        }
    };
}

function setupEventListeners() {
    document.querySelectorAll('.control-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            document.querySelectorAll('.control-btn').forEach(b => b.classList.remove('active'));
            e.target.classList.add('active');
            currentTrendingMetric = e.target.dataset.metric;
            fetchAndUpdate();
        });
    });
}

async function fetchAndUpdate() {
    try {
        const data = await fetchMetrics();
        if (data && data.length > 0) {
            updateDashboard(data);
            updateConnectionStatus(true);
        } else {
            updateConnectionStatus(false, 'No data');
        }
    } catch (error) {
        console.error('Fetch error:', error);
        updateConnectionStatus(false, 'Error');
    }
}

async function fetchMetrics() {
    const query = {
        size: 100,
        sort: [{ '@timestamp': 'desc' }],
        query: {
            range: {
                '@timestamp': {
                    gte: 'now-1h'
                }
            }
        }
    };

    const response = await fetch(`${ES_URL}/${ES_INDEX}/_search`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(query)
    });

    if (!response.ok) {
        throw new Error(`ES returned ${response.status}`);
    }

    const result = await response.json();
    return result.hits.hits.map(hit => hit._source).reverse();
}

function updateDashboard(data) {
    const latest = data[data.length - 1];

    // Update stat cards
    document.getElementById('total-hits').textContent = formatNumber(latest.hits);
    document.getElementById('total-misses').textContent = formatNumber(latest.misses);
    document.getElementById('total-items').textContent = formatNumber(latest.items);
    document.getElementById('memory-usage').textContent = formatBytes(latest.memory_bytes);

    // Update hit ratio badge
    const hitRatio = (latest.hit_ratio * 100).toFixed(1);
    document.getElementById('hit-ratio-badge').textContent = `${hitRatio}%`;

    // Update ops total badge
    const totalOps = latest.gets + latest.sets + latest.deletes;
    document.getElementById('ops-total').textContent = `${formatNumber(totalOps)} ops`;

    // Update Hit/Miss Chart
    hitMissChart.data.datasets[0].data = [latest.hits, latest.misses];
    hitMissChart.update('none');

    // Update Ops Chart
    opsChart.data.datasets[0].data = [latest.gets, latest.sets, latest.deletes];
    opsChart.update('none');

    // Update Trending Chart
    const labels = data.map(d => formatTime(d['@timestamp']));
    let trendingData, trendingLabel;

    switch (currentTrendingMetric) {
        case 'items':
            trendingData = data.map(d => d.items);
            trendingLabel = 'Cached Items';
            break;
        case 'memory':
            trendingData = data.map(d => d.memory_bytes / 1024);
            trendingLabel = 'Memory (KB)';
            break;
        default:
            trendingData = data.map(d => (d.hit_ratio * 100).toFixed(2));
            trendingLabel = 'Hit Ratio %';
    }

    trendingChart.data.labels = labels;
    trendingChart.data.datasets[0].data = trendingData;
    trendingChart.data.datasets[0].label = trendingLabel;
    trendingChart.update('none');

    // Update Ops Over Time Chart
    opsTimeChart.data.labels = labels;
    opsTimeChart.data.datasets[0].data = data.map(d => d.gets);
    opsTimeChart.data.datasets[1].data = data.map(d => d.sets);
    opsTimeChart.update('none');

    // Update last update time
    document.getElementById('last-update').textContent = new Date().toLocaleTimeString();
}

function updateConnectionStatus(connected, message = '') {
    const statusEl = document.getElementById('connection-status');
    const dotEl = document.querySelector('.status-dot');

    if (connected) {
        statusEl.textContent = 'Connected';
        dotEl.style.background = '#22c55e';
    } else {
        statusEl.textContent = message || 'Disconnected';
        dotEl.style.background = '#ef4444';
    }
}

function formatNumber(num) {
    if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
    if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
    return num.toString();
}

function formatBytes(bytes) {
    if (bytes >= 1073741824) return (bytes / 1073741824).toFixed(1) + ' GB';
    if (bytes >= 1048576) return (bytes / 1048576).toFixed(1) + ' MB';
    if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return bytes + ' B';
}

function formatTime(isoString) {
    const date = new Date(isoString);
    return date.toLocaleTimeString('en-US', { 
        hour: '2-digit', 
        minute: '2-digit',
        second: '2-digit'
    });
}
