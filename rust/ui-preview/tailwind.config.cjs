module.exports = {
  content: ['./src/**/*.rs', './.tailwind'],
  theme: {
    extend: {
      colors: {
        'oa-blue-lighter': '#5bb8dc',
        'oa-blue': '#0f7f9e',
        'oa-soft': '#e9f5f7',
        'oa-blue-darker': '#0a5e76',
        'oa-red': '#e52323',
        'oa-red-darker': '#be1717',
        'oa-gray': '#e6e6e6',
        'oa-gray-mid': '#d6d6d6',
        'oa-gray-darker': '#c3c3c3',
        success: '#16744a',
        'success-soft': '#e5f5ec',
        warning: '#995b09',
        'warning-soft': '#fff4d8',
        danger: '#a52d35',
        'danger-soft': '#fce9eb'
      },
      fontFamily: { sans: ['Inter', 'ui-sans-serif', 'system-ui'] }
    }
  }
};
