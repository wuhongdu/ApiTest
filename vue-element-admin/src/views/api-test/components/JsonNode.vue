<template>
  <div class="json-node" :style="{ paddingLeft: depth === 0 ? '0' : '14px' }">
    <div class="line">
      <span
        v-if="expandable"
        class="toggle"
        @click="expanded = !expanded"
      >{{ expanded ? '▼' : '▶' }}</span>
      <span v-else class="toggle placeholder" />
      <span v-if="name !== null && name !== undefined && depth > 0" class="key">{{ displayName }}: </span>
      <span v-if="!expandable" :class="valueClass">{{ displayValue }}</span>
      <button
        v-if="canSetToken"
        type="button"
        class="token-btn"
        title="抓取为 Token 并应用到集合"
        @click.stop="emitSetToken"
      >Token</button>
      <span v-else-if="expandable" class="preview" @click="expanded = !expanded">{{ preview }}</span>
    </div>
    <div v-if="expandable && expanded">
      <json-node
        v-for="(child, idx) in children"
        :key="idx"
        :name="child.name"
        :value="child.value"
        :depth="depth + 1"
        :default-expand-depth="defaultExpandDepth"
        @set-token="$emit('set-token', $event)"
      />
    </div>
  </div>
</template>

<script>
export default {
  name: 'JsonNode',
  props: {
    name: {
      default: null
    },
    value: {
      required: true
    },
    depth: {
      type: Number,
      default: 0
    },
    defaultExpandDepth: {
      type: Number,
      default: 2
    }
  },
  data() {
    return {
      expanded: this.depth < this.defaultExpandDepth
    }
  },
  computed: {
    type() {
      if (this.value === null) return 'null'
      if (Array.isArray(this.value)) return 'array'
      return typeof this.value
    },
    expandable() {
      return this.type === 'object' || this.type === 'array'
    },
    displayName() {
      return typeof this.name === 'number' ? this.name : `"${this.name}"`
    },
    displayValue() {
      if (this.type === 'string') return JSON.stringify(this.value)
      if (this.type === 'null') return 'null'
      return String(this.value)
    },
    valueClass() {
      return ['val', `val-${this.type}`]
    },
    preview() {
      if (this.type === 'array') return `Array(${this.value.length})`
      const keys = Object.keys(this.value || {})
      return `Object{${keys.length}}`
    },
    children() {
      if (this.type === 'array') {
        return this.value.map((v, i) => ({ name: i, value: v }))
      }
      if (this.type === 'object' && this.value) {
        return Object.keys(this.value).map(k => ({ name: k, value: this.value[k] }))
      }
      return []
    },
    canSetToken() {
      return this.type === 'string' && String(this.value || '').trim().length >= 4
    }
  },
  methods: {
    emitSetToken() {
      this.$emit('set-token', {
        value: String(this.value),
        key: typeof this.name === 'string' ? this.name : ''
      })
    }
  }
}
</script>

<style lang="scss" scoped>
.line {
  display: flex;
  align-items: flex-start;
  gap: 2px;
  min-height: 18px;
}

.toggle {
  width: 12px;
  cursor: pointer;
  color: #94a3b8;
  user-select: none;
  flex-shrink: 0;

  &.placeholder {
    cursor: default;
  }
}

.key {
  color: #7dd3fc;
}

.preview {
  color: #94a3b8;
  cursor: pointer;
}

.val-string {
  color: #86efac;
}

.val-number {
  color: #fcd34d;
}

.val-boolean {
  color: #c4b5fd;
}

.val-null {
  color: #fb7185;
}

.token-btn {
  margin-left: 6px;
  border: 1px solid #475569;
  background: #334155;
  color: #fbbf24;
  font-size: 10px;
  font-weight: 700;
  padding: 0 5px;
  height: 16px;
  line-height: 14px;
  border-radius: 3px;
  cursor: pointer;
  font-family: inherit;
  opacity: 0.55;
  flex-shrink: 0;

  .line:hover & {
    opacity: 1;
  }

  &:hover {
    background: #475569;
    border-color: #fbbf24;
    color: #fde68a;
  }
}
</style>
