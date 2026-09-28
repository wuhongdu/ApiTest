<template>
  <svg
    class="at-icon"
    :class="['at-icon--' + name, { spin: name === 'loading' }]"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <!-- plus -->
    <template v-if="name === 'plus'">
      <path d="M12 5v14M5 12h14" :stroke="stroke" stroke-width="1.75" stroke-linecap="round" />
    </template>

    <!-- close / x -->
    <template v-else-if="name === 'close'">
      <path d="M6 6l12 12M18 6L6 18" :stroke="stroke" stroke-width="1.75" stroke-linecap="round" />
    </template>

    <!-- env / variables (layers) -->
    <template v-else-if="name === 'env'">
      <path d="M4 8.5L12 4l8 4.5-8 4.5L4 8.5z" :stroke="stroke" stroke-width="1.6" stroke-linejoin="round" />
      <path d="M4 12.5l8 4.5 8-4.5M4 16.5l8 4.5 8-4.5" :stroke="stroke" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <!-- settings / gear -->
    <template v-else-if="name === 'settings'">
      <circle cx="12" cy="12" r="3" :stroke="stroke" stroke-width="1.6" />
      <path
        d="M12 3.5v2.2M12 18.3v2.2M4.9 6.5l1.6 1.6M17.5 15.9l1.6 1.6M3.5 12h2.2M18.3 12h2.2M4.9 17.5l1.6-1.6M17.5 8.1l1.6-1.6"
        :stroke="stroke"
        stroke-width="1.6"
        stroke-linecap="round"
      />
    </template>

    <!-- import (arrow into tray) -->
    <template v-else-if="name === 'import'">
      <path d="M12 3v11M8.5 10.5L12 14l3.5-3.5" :stroke="stroke" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M5 17.5v1.5a1 1 0 001 1h12a1 1 0 001-1v-1.5" :stroke="stroke" stroke-width="1.7" stroke-linecap="round" />
    </template>

    <!-- export (arrow out of tray) -->
    <template v-else-if="name === 'export'">
      <path d="M12 14V3M8.5 6.5L12 3l3.5 3.5" :stroke="stroke" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M5 17.5v1.5a1 1 0 001 1h12a1 1 0 001-1v-1.5" :stroke="stroke" stroke-width="1.7" stroke-linecap="round" />
    </template>

    <!-- sun -->
    <template v-else-if="name === 'sun'">
      <circle cx="12" cy="12" r="3.5" :stroke="stroke" stroke-width="1.6" />
      <path
        d="M12 3.5v2M12 18.5v2M4.2 4.2l1.4 1.4M18.4 18.4l1.4 1.4M3.5 12h2M18.5 12h2M4.2 19.8l1.4-1.4M18.4 5.6l1.4-1.4"
        :stroke="stroke"
        stroke-width="1.6"
        stroke-linecap="round"
      />
    </template>

    <!-- moon -->
    <template v-else-if="name === 'moon'">
      <path
        d="M18.5 14.2A7 7 0 019.8 5.5 7.2 7.2 0 0012 19a7.2 7.2 0 006.5-4.8z"
        :stroke="stroke"
        stroke-width="1.6"
        stroke-linejoin="round"
      />
    </template>

    <!-- folder -->
    <template v-else-if="name === 'folder'">
      <path
        d="M3.5 8.5V18a1.5 1.5 0 001.5 1.5h14A1.5 1.5 0 0020.5 18V9.5A1.5 1.5 0 0019 8H11L9.2 5.8A1 1 0 008.4 5.5H5A1.5 1.5 0 003.5 7v1.5z"
        :stroke="stroke"
        stroke-width="1.6"
        stroke-linejoin="round"
      />
    </template>

    <!-- folder-plus -->
    <template v-else-if="name === 'folder-plus'">
      <path
        d="M3.5 8.5V18a1.5 1.5 0 001.5 1.5h14A1.5 1.5 0 0020.5 18V9.5A1.5 1.5 0 0019 8H11L9.2 5.8A1 1 0 008.4 5.5H5A1.5 1.5 0 003.5 7v1.5z"
        :stroke="stroke"
        stroke-width="1.5"
        stroke-linejoin="round"
      />
      <path d="M12 11.5v5M9.5 14h5" :stroke="stroke" stroke-width="1.5" stroke-linecap="round" />
    </template>

    <!-- request / document-plus -->
    <template v-else-if="name === 'request'">
      <path
        d="M7 3.5h6.5L17.5 8v12a1 1 0 01-1 1H7a1 1 0 01-1-1V4.5a1 1 0 011-1z"
        :stroke="stroke"
        stroke-width="1.6"
        stroke-linejoin="round"
      />
      <path d="M13.5 3.5V8h4.5M9.5 13h5M9.5 16.5h3.5" :stroke="stroke" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <!-- refresh -->
    <template v-else-if="name === 'refresh'">
      <path
        d="M19.5 12a7.5 7.5 0 11-2.2-5.3"
        :stroke="stroke"
        stroke-width="1.7"
        stroke-linecap="round"
      />
      <path d="M19.5 4.5v5h-5" :stroke="stroke" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <!-- edit / pencil -->
    <template v-else-if="name === 'edit'">
      <path
        d="M14.2 5.3l4.5 4.5M5 19l1.2-4.6L15.8 4.8a1.5 1.5 0 012.1 0l1.3 1.3a1.5 1.5 0 010 2.1L8.6 18.8 4 20l1-1z"
        :stroke="stroke"
        stroke-width="1.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>

    <!-- trash -->
    <template v-else-if="name === 'trash'">
      <path
        d="M5 7.5h14M9.5 7.5V5.8A1 1 0 0110.5 5h3a1 1 0 011 1v1.5M8 7.5l.7 11a1 1 0 001 1h4.6a1 1 0 001-1l.7-11"
        :stroke="stroke"
        stroke-width="1.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>

    <!-- copy -->
    <template v-else-if="name === 'copy'">
      <rect x="8" y="8" width="11" height="11" rx="1.5" :stroke="stroke" stroke-width="1.6" />
      <path d="M5.5 15.5V5.5A1.5 1.5 0 017 4h10" :stroke="stroke" stroke-width="1.6" stroke-linecap="round" />
    </template>

    <!-- send / paper plane -->
    <template v-else-if="name === 'send'">
      <path
        d="M20.5 3.5L3.5 10.8l7.2 2.5 2.5 7.2L20.5 3.5zM10.7 13.3l4.6-4.6"
        :stroke="stroke"
        stroke-width="1.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>

    <!-- check -->
    <template v-else-if="name === 'check'">
      <path d="M5 12.5l4.5 4.5L19 7.5" :stroke="stroke" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <!-- x-circle / fail -->
    <template v-else-if="name === 'fail'">
      <circle cx="12" cy="12" r="8" :stroke="stroke" stroke-width="1.6" />
      <path d="M9 9l6 6M15 9l-6 6" :stroke="stroke" stroke-width="1.6" stroke-linecap="round" />
    </template>

    <!-- check-circle / pass -->
    <template v-else-if="name === 'pass'">
      <circle cx="12" cy="12" r="8" :stroke="stroke" stroke-width="1.6" />
      <path d="M8 12.2l2.8 2.8L16.2 9.5" :stroke="stroke" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <!-- loading (spinner arc) -->
    <template v-else-if="name === 'loading'">
      <path
        d="M12 3.5a8.5 8.5 0 11-7.4 4.3"
        :stroke="stroke"
        stroke-width="1.8"
        stroke-linecap="round"
      />
    </template>

    <!-- window minimize -->
    <template v-else-if="name === 'win-min'">
      <path d="M5 12.5h14" :stroke="stroke" stroke-width="1.7" stroke-linecap="round" />
    </template>

    <!-- window maximize -->
    <template v-else-if="name === 'win-max'">
      <rect x="5.5" y="5.5" width="13" height="13" rx="1.2" :stroke="stroke" stroke-width="1.6" />
    </template>

    <!-- window restore -->
    <template v-else-if="name === 'win-restore'">
      <path d="M8 8.5h9.5V18H8V8.5z" :stroke="stroke" stroke-width="1.5" />
      <path d="M10.5 8.5V6.5H20v9.5h-2" :stroke="stroke" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <!-- window close -->
    <template v-else-if="name === 'win-close'">
      <path d="M6.5 6.5l11 11M17.5 6.5l-11 11" :stroke="stroke" stroke-width="1.7" stroke-linecap="round" />
    </template>

    <!-- api / link nodes -->
    <template v-else-if="name === 'api'">
      <circle cx="7" cy="12" r="2.5" :stroke="stroke" stroke-width="1.6" />
      <circle cx="17" cy="7" r="2.5" :stroke="stroke" stroke-width="1.6" />
      <circle cx="17" cy="17" r="2.5" :stroke="stroke" stroke-width="1.6" />
      <path d="M9.3 11.2l5-3.2M9.3 12.8l5 3.2" :stroke="stroke" stroke-width="1.6" stroke-linecap="round" />
    </template>

    <!-- history clock -->
    <template v-else-if="name === 'history'">
      <circle cx="12" cy="13" r="7" :stroke="stroke" stroke-width="1.6" />
      <path d="M12 13V9.5M12 13l3 2M9 4.5h6" :stroke="stroke" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <!-- fallback: circle -->
    <template v-else>
      <circle cx="12" cy="12" r="7" :stroke="stroke" stroke-width="1.6" />
    </template>
  </svg>
</template>

<script>
export default {
  name: 'AtIcon',
  props: {
    name: {
      type: String,
      required: true
    },
    size: {
      type: [Number, String],
      default: 16
    },
    stroke: {
      type: String,
      default: 'currentColor'
    }
  }
}
</script>

<style scoped>
.at-icon {
  display: inline-block;
  vertical-align: -0.2em;
  flex-shrink: 0;
  color: inherit;
}

.at-icon.spin {
  animation: at-icon-spin 0.8s linear infinite;
}

@keyframes at-icon-spin {
  to { transform: rotate(360deg); }
}
</style>
