<template>
  <div class="json-viewer">
    <div class="jv-toolbar">
      <div class="jv-left">
        <template v-if="isJson">
          <button
            type="button"
            class="jv-mode"
            :class="{ active: viewMode === 'pretty' }"
            @click="viewMode = 'pretty'"
          >Pretty</button>
          <button
            type="button"
            class="jv-mode"
            :class="{ active: viewMode === 'raw' }"
            @click="viewMode = 'raw'"
          >Raw</button>
        </template>
        <span v-else class="jv-badge">Text</span>
        <span v-if="byteLabel" class="jv-meta">{{ byteLabel }}</span>
      </div>
      <div class="jv-right">
        <button
          v-if="isJson && viewMode === 'pretty'"
          type="button"
          class="jv-action"
          title="全部展开"
          @click="expandDepth = 99; treeKey++"
        >Expand</button>
        <button
          v-if="isJson && viewMode === 'pretty'"
          type="button"
          class="jv-action"
          title="全部折叠"
          @click="expandDepth = 1; treeKey++"
        >Collapse</button>
        <button
          type="button"
          class="jv-action"
          :disabled="!text"
          title="复制响应体"
          @click="onCopy"
        >Copy</button>
      </div>
    </div>

    <div class="jv-body">
      <div v-if="!text" class="jv-empty">空响应体</div>
      <pre v-else-if="!isJson || viewMode === 'raw'" class="jv-raw">{{ displayRaw }}</pre>
      <json-node
        v-else
        :key="treeKey"
        :value="parsed"
        :name="rootName"
        :depth="0"
        :default-expand-depth="expandDepth"
      />
    </div>
  </div>
</template>

<script>
import JsonNode from './JsonNode.vue'

export default {
  name: 'JsonViewer',
  components: { JsonNode },
  props: {
    text: {
      type: String,
      default: ''
    },
    rootName: {
      type: String,
      default: 'root'
    }
  },
  data() {
    return {
      viewMode: 'pretty',
      expandDepth: 2,
      treeKey: 0
    }
  },
  computed: {
    parsed() {
      if (!this.text || !this.text.trim()) return null
      try {
        return JSON.parse(this.text)
      } catch (e) {
        return null
      }
    },
    isJson() {
      return this.parsed !== null
    },
    displayRaw() {
      if (this.isJson) {
        try {
          return JSON.stringify(this.parsed, null, 2)
        } catch (e) {
          return this.text
        }
      }
      return this.text
    },
    byteLabel() {
      if (!this.text) return ''
      const n = typeof TextEncoder !== 'undefined'
        ? new TextEncoder().encode(this.text).length
        : this.text.length
      if (n < 1024) return `${n} B`
      if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
      return `${(n / (1024 * 1024)).toFixed(2)} MB`
    }
  },
  watch: {
    text() {
      this.viewMode = 'pretty'
      this.expandDepth = 2
      this.treeKey++
    }
  },
  methods: {
    async onCopy() {
      const value = this.viewMode === 'raw' || !this.isJson ? this.displayRaw : this.displayRaw
      try {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          await navigator.clipboard.writeText(value)
        } else {
          const ta = document.createElement('textarea')
          ta.value = value
          ta.style.position = 'fixed'
          ta.style.left = '-9999px'
          document.body.appendChild(ta)
          ta.select()
          document.execCommand('copy')
          document.body.removeChild(ta)
        }
        this.$message.success('已复制')
      } catch (e) {
        this.$message.error('复制失败')
      }
    }
  }
}
</script>

<style lang="scss" scoped>
.json-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 160px;
  font-family: ui-monospace, Menlo, Consolas, 'Courier New', monospace;
  font-size: 12px;
  line-height: 1.55;
  background: #0f172a;
  color: #e2e8f0;
  border-radius: 6px;
  overflow: hidden;
}

.jv-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 8px;
  background: #1e293b;
  border-bottom: 1px solid #334155;
  flex-shrink: 0;
}

.jv-left,
.jv-right {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
}

.jv-mode,
.jv-action {
  border: 1px solid transparent;
  background: transparent;
  color: #94a3b8;
  font-size: 11px;
  font-weight: 600;
  padding: 3px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-family: inherit;

  &:hover:not(:disabled) {
    color: #e2e8f0;
    background: #334155;
  }

  &:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  &.active {
    color: #fff;
    background: #475569;
    border-color: #64748b;
  }
}

.jv-badge {
  font-size: 11px;
  font-weight: 700;
  color: #94a3b8;
  padding: 2px 8px;
  background: #334155;
  border-radius: 4px;
}

.jv-meta {
  font-size: 11px;
  color: #64748b;
  margin-left: 4px;
  white-space: nowrap;
}

.jv-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 10px 12px;
}

.jv-raw {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: inherit;
  font-size: inherit;
  line-height: inherit;
  color: inherit;
}

.jv-empty {
  color: #64748b;
  text-align: center;
  padding: 32px 12px;
  font-size: 12px;
}
</style>
